//! Regex matcher.

use super::Matcher;
use crate::error::SelError;
use crate::{Line, MatchInfo, Result};
use regex::bytes::Regex;

#[derive(Debug)]
pub struct RegexMatcher {
    regex: Regex,
    invert: bool,
}

impl RegexMatcher {
    pub fn new(pattern: &str, invert: bool) -> Result<Self> {
        let regex = Regex::new(pattern).map_err(|e| SelError::InvalidRegex(e.to_string()))?;
        Ok(Self { regex, invert })
    }
}

impl Matcher for RegexMatcher {
    fn match_line(&mut self, line: &Line) -> MatchInfo {
        let is_match = self.regex.is_match(&line.bytes);
        let hit = is_match ^ self.invert;
        // Inverted hits have nothing to highlight.
        let spans = if hit && !self.invert {
            self.regex
                .find_iter(&line.bytes)
                .map(|m| m.start()..m.end())
                .collect()
        } else {
            Vec::new()
        };
        MatchInfo {
            hit,
            spans,
            col: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(bytes: &[u8]) -> Line {
        Line::new(1, bytes.to_vec())
    }

    #[test]
    fn basic_match_gives_spans() {
        let mut m = RegexMatcher::new("ERROR", false).unwrap();
        let info = m.match_line(&mk(b"an ERROR happened"));
        assert!(info.hit);
        assert_eq!(info.spans.len(), 1);
        assert_eq!(info.spans[0], 3..8);
    }

    #[test]
    fn invert_flips_hit_and_clears_spans() {
        let mut m = RegexMatcher::new("ERROR", true).unwrap();
        let miss_inverted = m.match_line(&mk(b"an ERROR happened"));
        assert!(!miss_inverted.hit);
        assert!(miss_inverted.spans.is_empty());

        let hit_inverted = m.match_line(&mk(b"all clear"));
        assert!(hit_inverted.hit);
        assert!(hit_inverted.spans.is_empty());
    }

    #[test]
    fn invalid_regex_errors() {
        let err = RegexMatcher::new("(unclosed", false).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("invalid regex"));
    }
}
