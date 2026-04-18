//! Shared types used across pipeline stages.

use std::ops::Range;

/// A line of input with its 1-indexed line number.
#[derive(Debug, Clone)]
pub struct Line {
    pub no: u64,
    pub bytes: Vec<u8>,
}

impl Line {
    pub fn new(no: u64, bytes: Vec<u8>) -> Self {
        Self { no, bytes }
    }

    /// Borrow the line content as a string, substituting U+FFFD for invalid UTF-8.
    pub fn as_str_lossy(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.bytes)
    }
}

/// Result of running a `Matcher` on a `Line`.
#[derive(Debug, Default, Clone)]
pub struct MatchInfo {
    /// Did this line hit?
    pub hit: bool,
    /// Byte ranges to highlight (for regex matches).
    pub spans: Vec<Range<usize>>,
    /// Target column (1-indexed) for positional matches.
    pub col: Option<usize>,
}

/// Role of an emitted line in the output stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// A line that matched (primary output).
    Target,
    /// A neighbouring line included as context.
    Context,
}

/// One line being emitted by the pipeline.
#[derive(Debug, Clone)]
pub struct Emit<'a> {
    pub line: &'a Line,
    pub role: Role,
    pub match_info: &'a MatchInfo,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_as_str_lossy_handles_invalid_utf8() {
        let line = Line::new(1, vec![0xFF, b'a']);
        let s = line.as_str_lossy();
        assert!(s.ends_with('a'));
    }

    #[test]
    fn match_info_default_is_miss() {
        let mi = MatchInfo::default();
        assert!(!mi.hit);
        assert!(mi.spans.is_empty());
        assert!(mi.col.is_none());
    }
}
