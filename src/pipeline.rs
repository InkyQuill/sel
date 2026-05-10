//! Single driver for all pipelines.

use crate::Emit;
use crate::Result;
use crate::app::{App, SourceKind};
use crate::context::EmitOwned;
use crate::sink::Sink;
use std::io;

pub fn run<K: SourceKind>(mut app: App<K>) -> Result<()> {
    process(&mut app)?;
    // Destructure and finalize the sink.
    let App { sink, .. } = app;
    finish_sink(sink)
}

pub fn run_unfinished<K: SourceKind>(mut app: App<K>) -> Result<Box<dyn Sink>> {
    process(&mut app)?;
    Ok(app.sink)
}

fn process<K: SourceKind>(app: &mut App<K>) -> Result<()> {
    // Read lines, run matcher, feed expander, write via formatter.
    let mut write_error: Option<io::Error> = None;
    while let Some(line) = app.source.next_line()? {
        let info = app.matcher.match_line(&line);
        let formatter = &mut app.formatter;
        let sink = &mut app.sink;
        app.expander.push(line, info, &mut |emit: EmitOwned| {
            if write_error.is_some() {
                return;
            }
            let borrowed = Emit {
                line: &emit.line,
                role: emit.role,
                match_info: &emit.match_info,
            };
            if let Err(err) = formatter.write(sink.as_mut(), &borrowed) {
                write_error = Some(err);
            }
        });
        if write_error.is_some() {
            break;
        }
    }
    let formatter = &mut app.formatter;
    let sink = &mut app.sink;
    if write_error.is_none() {
        app.expander.drain(&mut |emit: EmitOwned| {
            if write_error.is_some() {
                return;
            }
            let borrowed = Emit {
                line: &emit.line,
                role: emit.role,
                match_info: &emit.match_info,
            };
            if let Err(err) = formatter.write(sink.as_mut(), &borrowed) {
                write_error = Some(err);
            }
        });
    }
    if let Some(source) = write_error {
        return Err(crate::SelError::Io {
            path: "<sink>".to_string(),
            source,
        });
    }
    Ok(())
}

pub fn finish_sink(sink: Box<dyn Sink>) -> Result<()> {
    sink.finish().map_err(|source| crate::SelError::Io {
        path: "<sink>".to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Stage1;
    use crate::context::NoContext;
    use crate::format::{FormatOpts, PlainFormatter};
    use crate::matcher::AllMatcher;
    use crate::sink::Sink;
    use crate::source::Source;
    use crate::{Line, SelError};
    use std::io::{self, Write};

    struct OneLineSource(Option<Line>);

    impl Source for OneLineSource {
        fn next_line(&mut self) -> crate::Result<Option<Line>> {
            Ok(self.0.take())
        }

        fn label(&self) -> &str {
            "one-line"
        }

        fn is_seekable(&self) -> bool {
            false
        }
    }

    struct FailingSink;

    impl Write for FailingSink {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "sink failed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl Sink for FailingSink {
        fn is_terminal(&self) -> bool {
            false
        }

        fn finish(self: Box<Self>) -> io::Result<()> {
            panic!("finish should not run after a write error");
        }
    }

    #[test]
    fn formatter_write_errors_are_returned() {
        let opts = FormatOpts {
            show_line_numbers: true,
            show_filename: false,
            filename: None,
            color: false,
            target_marker: false,
            line_number_width: 4,
        };
        let app = Stage1::with_nonseekable_source(Box::new(OneLineSource(Some(Line::new(
            1,
            b"line".to_vec(),
        )))))
        .with_matcher(Box::new(AllMatcher))
        .with_expander(Box::new(NoContext))
        .with_formatter(Box::new(PlainFormatter::new(opts)))
        .with_sink(Box::new(FailingSink));

        let err = run(app).unwrap_err();
        match err {
            SelError::Io { source, .. } => {
                assert_eq!(source.kind(), io::ErrorKind::BrokenPipe);
            }
            other => panic!("expected io error, got {other:?}"),
        }
    }
}
