//! Plain-line formatter with optional line number, filename, and highlight.

use super::{FormatOpts, Formatter, ansi};
use crate::{Emit, Role};
use std::io::{self, Write};
use std::ops::Range;

pub struct PlainFormatter {
    pub opts: FormatOpts,
}

impl PlainFormatter {
    pub fn new(opts: FormatOpts) -> Self {
        Self { opts }
    }

    fn render_content(&self, bytes: &[u8], spans: &[Range<usize>]) -> String {
        let text = String::from_utf8_lossy(bytes);
        if !self.opts.color || spans.is_empty() {
            return text.to_string();
        }
        let mut sorted = spans.to_vec();
        sorted.sort_by_key(|r| r.start);
        let mut out = String::new();
        let mut cursor = 0usize;
        let t: &str = text.as_ref();
        for r in sorted {
            let s = r.start.min(t.len());
            let e = r.end.min(t.len());
            if s < cursor {
                continue;
            }
            out.push_str(&t[cursor..s]);
            out.push_str(ansi::INVERSE);
            out.push_str(&t[s..e]);
            out.push_str(ansi::RESET);
            cursor = e;
        }
        out.push_str(&t[cursor..]);
        out
    }
}

impl Formatter for PlainFormatter {
    fn write(&mut self, sink: &mut dyn Write, emit: &Emit) -> io::Result<()> {
        let marker = match emit.role {
            Role::Target if self.opts.target_marker => {
                ansi::paint(self.opts.color, ansi::GREEN, ">") + " "
            }
            _ => String::new(),
        };
        let prefix = self.opts.prefix(emit.line.no);
        let content = self.render_content(&emit.line.bytes, &emit.match_info.spans);
        writeln!(sink, "{marker}{prefix}{content}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Line, MatchInfo};

    fn opts(color: bool) -> FormatOpts {
        FormatOpts {
            show_line_numbers: true,
            show_filename: false,
            filename: None,
            color,
            target_marker: true,
        }
    }

    #[test]
    fn target_gets_marker_and_prefix() {
        let line = Line::new(7, b"hello".to_vec());
        let mi = MatchInfo {
            hit: true,
            ..Default::default()
        };
        let emit = Emit {
            line: &line,
            role: Role::Target,
            match_info: &mi,
        };
        let mut f = PlainFormatter::new(opts(false));
        let mut buf: Vec<u8> = Vec::new();
        f.write(&mut buf, &emit).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "> 7:hello\n");
    }

    #[test]
    fn context_has_no_marker() {
        let line = Line::new(3, b"ctx".to_vec());
        let mi = MatchInfo::default();
        let emit = Emit {
            line: &line,
            role: Role::Context,
            match_info: &mi,
        };
        let mut f = PlainFormatter::new(opts(false));
        let mut buf: Vec<u8> = Vec::new();
        f.write(&mut buf, &emit).unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), "3:ctx\n");
    }

    #[test]
    #[allow(clippy::single_range_in_vec_init)]
    fn spans_highlight_when_color_enabled() {
        let line = Line::new(1, b"an ERROR today".to_vec());
        let mi = MatchInfo {
            hit: true,
            spans: vec![3..8],
            ..Default::default()
        };
        let emit = Emit {
            line: &line,
            role: Role::Target,
            match_info: &mi,
        };
        let mut f = PlainFormatter::new(opts(true));
        let mut buf: Vec<u8> = Vec::new();
        f.write(&mut buf, &emit).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("\x1b[7mERROR\x1b[0m"));
    }
}
