//! Input sources — iterators that yield `Line`s one at a time.

pub mod file;
pub mod stdin;

pub use file::FileSource;
pub use stdin::StdinSource;

use crate::Line;
use crate::Result;

/// A stream of input lines with 1-indexed line numbers.
///
/// Implementations own the underlying reader and handle newline stripping.
pub trait Source {
    /// Read the next line. Returns `Ok(None)` at EOF.
    fn next_line(&mut self) -> Result<Option<Line>>;

    /// A short label for this source, used for the filename prefix (`-` for stdin).
    fn label(&self) -> &str;

    /// Can this source be paired with positional selectors?
    /// `false` for stdin.
    fn is_seekable(&self) -> bool;
}
