//! Fragment formatter — char-context window with caret.
//!
//! Used when `-n` is set (positional selectors or `-e` + `-n`).

use super::{FormatOpts, Formatter, ansi};
use crate::Emit;
use std::io::{self, Write};

pub struct FragmentFormatter {
    pub opts: FormatOpts,
    pub char_context: usize,
}

impl FragmentFormatter {
    pub fn new(opts: FormatOpts, char_context: usize) -> Self {
        Self { opts, char_context }
    }
}

impl Formatter for FragmentFormatter {
    fn write(&mut self, sink: &mut dyn Write, emit: &Emit) -> io::Result<()> {
        self.opts.widen_for_line(emit.line.no);
        // Target column: from position matcher (`col`), else start of first regex span.
        let target_col_1 = emit
            .match_info
            .col
            .or_else(|| emit.match_info.spans.first().map(|r| r.start + 1))
            .unwrap_or(1);

        let bytes = &emit.line.bytes;
        let col_idx = target_col_1
            .saturating_sub(1)
            .min(bytes.len().saturating_sub(1));
        let start = col_idx.saturating_sub(self.char_context);
        let end = bytes.len().min(col_idx + self.char_context + 1);

        let frag = String::from_utf8_lossy(&bytes[start..end]).to_string();
        let prefix = self.opts.prefix(emit.line.no);

        // Highlight the target span within the fragment if regex spans exist.
        let rendered = if let Some(span) = emit.match_info.spans.first() {
            if self.opts.color {
                let hs = span.start.saturating_sub(start).min(frag.len());
                let he = (span.end.saturating_sub(start)).min(frag.len());
                if hs < he {
                    let (a, rest) = frag.split_at(hs);
                    let (b, c) = rest.split_at(he - hs);
                    format!("{a}{}{b}{}{c}", ansi::INVERSE, ansi::RESET)
                } else {
                    frag.clone()
                }
            } else {
                frag.clone()
            }
        } else {
            frag.clone()
        };

        writeln!(sink, "{prefix}{rendered}")?;

        // Caret line, aligned under the target column within the fragment.
        let caret_offset = col_idx - start + prefix.len();
        let spaces = " ".repeat(caret_offset);
        let caret = ansi::paint(self.opts.color, ansi::GREEN, "^");
        writeln!(sink, "{spaces}{caret}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Line, MatchInfo, Role};

    #[test]
    fn renders_fragment_with_caret_under_col() {
        let line = Line::new(1, b"abcdefghij".to_vec());
        let mi = MatchInfo {
            hit: true,
            col: Some(5),
            ..Default::default()
        };
        let emit = Emit {
            line: &line,
            role: Role::Target,
            match_info: &mi,
        };
        let opts = FormatOpts {
            show_line_numbers: true,
            show_filename: false,
            filename: None,
            color: false,
            target_marker: false,
            line_number_width: 4,
        };
        let mut f = FragmentFormatter::new(opts, 2);
        let mut buf: Vec<u8> = Vec::new();
        f.write(&mut buf, &emit).unwrap();
        let s = String::from_utf8(buf).unwrap();
        // Fragment: col=5 with context=2 → bytes [2..7] = "cdefg"
        // Caret at col 5 → offset 2 in fragment
        assert_eq!(s, "   1: cdefg\n        ^\n");
    }
}
