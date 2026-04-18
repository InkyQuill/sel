//! Line-number matcher built from sorted, merged ranges.

use super::Matcher;
use crate::selector::{LineSpec, Selector};
use crate::{Line, MatchInfo};

/// Matches lines whose 1-indexed number falls in a set of merged ranges.
pub struct LineMatcher {
    /// Sorted, non-overlapping, inclusive `(start, end)` ranges (1-indexed).
    ranges: Vec<(u64, u64)>,
}

impl LineMatcher {
    /// Build from a `Selector::LineNumbers`. Panics on other variants.
    pub fn from_selector(sel: &Selector) -> Self {
        let normalized = sel.normalize();
        let specs: &[LineSpec] = match &normalized {
            Selector::LineNumbers(s) => s,
            Selector::All => &[],
            Selector::Positions(_) => {
                panic!("LineMatcher::from_selector called with positional selector")
            }
        };
        let ranges = specs
            .iter()
            .map(|s| match s {
                LineSpec::Single(n) => (*n as u64, *n as u64),
                LineSpec::Range(a, b) => (*a as u64, *b as u64),
            })
            .collect();
        Self { ranges }
    }
}

impl Matcher for LineMatcher {
    fn match_line(&mut self, line: &Line) -> MatchInfo {
        let hit = self
            .ranges
            .iter()
            .any(|&(a, b)| line.no >= a && line.no <= b);
        MatchInfo {
            hit,
            ..MatchInfo::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_line(n: u64) -> Line {
        Line::new(n, Vec::new())
    }

    #[test]
    fn single_line_hits_only_that_line() {
        let sel = Selector::parse("5").unwrap();
        let mut m = LineMatcher::from_selector(&sel);
        assert!(!m.match_line(&mk_line(4)).hit);
        assert!(m.match_line(&mk_line(5)).hit);
        assert!(!m.match_line(&mk_line(6)).hit);
    }

    #[test]
    fn range_hits_inclusive() {
        let sel = Selector::parse("10-12").unwrap();
        let mut m = LineMatcher::from_selector(&sel);
        assert!(!m.match_line(&mk_line(9)).hit);
        assert!(m.match_line(&mk_line(10)).hit);
        assert!(m.match_line(&mk_line(11)).hit);
        assert!(m.match_line(&mk_line(12)).hit);
        assert!(!m.match_line(&mk_line(13)).hit);
    }

    #[test]
    fn mixed_list_merges_ranges() {
        let sel = Selector::parse("1,5,10-15,14").unwrap();
        let mut m = LineMatcher::from_selector(&sel);
        assert!(m.match_line(&mk_line(1)).hit);
        assert!(!m.match_line(&mk_line(2)).hit);
        assert!(m.match_line(&mk_line(5)).hit);
        assert!(m.match_line(&mk_line(12)).hit);
        assert!(m.match_line(&mk_line(15)).hit);
        assert!(!m.match_line(&mk_line(16)).hit);
    }
}
