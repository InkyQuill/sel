//! Matcher stage — decides whether a given line is a hit.

pub mod lines;
pub mod position;
pub mod regex;

pub use self::regex::RegexMatcher;
pub use lines::LineMatcher;
pub use position::PositionMatcher;

use crate::{Line, MatchInfo};

/// Classifies each line as hit or miss.
///
/// Implementations may be stateful (e.g., a position matcher that advances
/// through a sorted list of targets).
pub trait Matcher {
    fn match_line(&mut self, line: &Line) -> MatchInfo;
}

/// A matcher that hits every line.
pub struct AllMatcher;

impl Matcher for AllMatcher {
    fn match_line(&mut self, _line: &Line) -> MatchInfo {
        MatchInfo {
            hit: true,
            ..MatchInfo::default()
        }
    }
}
