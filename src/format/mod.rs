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
}

impl FormatOpts {
    pub fn prefix(&self, line_no: u64) -> String {
        let mut p = String::new();
        if self.show_filename
            && let Some(f) = &self.filename
        {
            p.push_str(f);
            p.push(':');
        }
        if self.show_line_numbers {
            p.push_str(&line_no.to_string());
            p.push(':');
        }
        p
    }
}
