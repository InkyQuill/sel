//! Single driver for all pipelines.

use crate::Emit;
use crate::Result;
use crate::app::{App, SourceKind};
use crate::context::EmitOwned;

pub fn run<K: SourceKind>(mut app: App<K>) -> Result<()> {
    // Read lines, run matcher, feed expander, write via formatter.
    while let Some(line) = app.source.next_line()? {
        let info = app.matcher.match_line(&line);
        let formatter = &mut app.formatter;
        let sink = &mut app.sink;
        app.expander.push(line, info, &mut |emit: EmitOwned| {
            let borrowed = Emit {
                line: &emit.line,
                role: emit.role,
                match_info: &emit.match_info,
            };
            let _ = formatter.write(sink.as_mut(), &borrowed);
        });
    }
    let formatter = &mut app.formatter;
    let sink = &mut app.sink;
    app.expander.drain(&mut |emit: EmitOwned| {
        let borrowed = Emit {
            line: &emit.line,
            role: emit.role,
            match_info: &emit.match_info,
        };
        let _ = formatter.write(sink.as_mut(), &borrowed);
    });
    // Destructure and finalize the sink.
    let App { sink, .. } = app;
    sink.finish().map_err(|source| crate::SelError::Io {
        path: "<sink>".to_string(),
        source,
    })
}
