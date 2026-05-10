//! Output formatting.

pub mod ansi;
pub mod fragment;
pub mod plain;

pub use fragment::FragmentFormatter;
pub use plain::PlainFormatter;

use crate::Emit;
use std::io;

/// A formatter serializes one `Emit` into bytes.
pub trait Formatter {
    fn write(&mut self, sink: &mut dyn io::Write, emit: &Emit) -> io::Result<()>;
}

/// Common configuration shared by plain and fragment formatters.
#[derive(Debug, Clone)]
pub struct FormatOpts {
    pub show_line_numbers: bool,
    pub show_filename: bool,
    pub filename: Option<String>,
    pub color: bool,
    /// Prepend `"> "` (colorized green) before target lines.
    /// Set `true` only when mixing target and context lines (i.e. `-c N`).
    pub target_marker: bool,
    pub line_number_width: usize,
}

impl FormatOpts {
    pub fn widen_for_line(&mut self, line_no: u64) {
        self.line_number_width = self.line_number_width.max(digits(line_no));
    }

    pub fn prefix(&self, line_no: u64) -> String {
        let mut p = String::new();
        if self.show_filename
            && let Some(f) = &self.filename
        {
            p.push_str(f);
            p.push(':');
        }
        if self.show_line_numbers {
            p.push_str(&format!(
                "{line_no:>width$}: ",
                width = self.line_number_width
            ));
        }
        p
    }
}

pub fn digits(n: u64) -> usize {
    n.to_string().len()
}
