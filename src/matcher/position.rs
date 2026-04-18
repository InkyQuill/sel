//! Positional matcher (line:column).

use super::Matcher;
use crate::selector::{Position, Selector};
use crate::{Line, MatchInfo};

pub struct PositionMatcher {
    /// Positions sorted by `(line, col)`.
    positions: Vec<Position>,
    /// Index of next unconsumed position (positions with line < current are skipped).
    cursor: usize,
}

impl PositionMatcher {
    /// Panics if `sel` is not `Selector::Positions`.
    pub fn from_selector(sel: &Selector) -> Self {
        let positions = match sel.normalize() {
            Selector::Positions(mut p) => {
                p.sort();
                p.dedup();
                p
            }
            _ => panic!("PositionMatcher::from_selector needs Selector::Positions"),
        };
        Self {
            positions,
            cursor: 0,
        }
    }
}

impl Matcher for PositionMatcher {
    fn match_line(&mut self, line: &Line) -> MatchInfo {
        // Advance cursor past any positions for earlier lines.
        while self.cursor < self.positions.len()
            && (self.positions[self.cursor].line as u64) < line.no
        {
            self.cursor += 1;
        }
        // First position on this line (if any) becomes the target column.
        if let Some(p) = self.positions.get(self.cursor)
            && p.line as u64 == line.no
        {
            return MatchInfo {
                hit: true,
                spans: Vec::new(),
                col: Some(p.column),
            };
        }
        MatchInfo::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_line(n: u64) -> Line {
        Line::new(n, Vec::new())
    }

    #[test]
    fn single_position_hits_correct_line_with_col() {
        let sel = Selector::parse("23:260").unwrap();
        let mut m = PositionMatcher::from_selector(&sel);
        assert!(!m.match_line(&mk_line(22)).hit);
        let info = m.match_line(&mk_line(23));
        assert!(info.hit);
        assert_eq!(info.col, Some(260));
        assert!(!m.match_line(&mk_line(24)).hit);
    }

    #[test]
    fn multiple_positions_hit_in_order() {
        let sel = Selector::parse("5:10,5:20,9:3").unwrap();
        let mut m = PositionMatcher::from_selector(&sel);
        let h5 = m.match_line(&mk_line(5));
        assert!(h5.hit);
        assert_eq!(h5.col, Some(10));
        let h9 = m.match_line(&mk_line(9));
        assert!(h9.hit);
        assert_eq!(h9.col, Some(3));
    }
}
