//! Selector parsing and representation.

use crate::error::{Result, SelError};
use std::collections::BTreeSet;

/// A selector specifying which lines/positions to extract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    /// Select all lines (no selector provided).
    All,

    /// Select specific line numbers and/or ranges.
    LineNumbers(Vec<LineSpec>),

    /// Select specific positions (line:column).
    Positions(Vec<Position>),
}

/// Merge overlapping or adjacent ranges.
fn merge_ranges(mut ranges: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    if ranges.is_empty() {
        return ranges;
    }

    ranges.sort_by_key(|&(start, _)| start);
    let mut merged = Vec::new();
    let mut current_start = ranges[0].0;
    let mut current_end = ranges[0].1;

    for &(start, end) in &ranges[1..] {
        if start <= current_end + 1 {
            // Overlapping or adjacent ranges - merge them
            current_end = current_end.max(end);
        } else {
            merged.push((current_start, current_end));
            current_start = start;
            current_end = end;
        }
    }
    merged.push((current_start, current_end));

    merged
}

/// Merge single lines into ranges.
///
/// Converts single lines into Range specs where they are contiguous
/// or adjacent to existing ranges.
fn merge_lines_with_ranges(lines: Vec<usize>, ranges: Vec<(usize, usize)>) -> Vec<LineSpec> {
    // Collect all "events": start and end of ranges, plus single lines
    // For a single line n, we treat it as a range (n, n)
    let mut all_ranges: Vec<(usize, usize)> = ranges;

    // Add single lines as ranges of length 1
    for line in lines {
        all_ranges.push((line, line));
    }

    // Merge all ranges
    let merged = merge_ranges(all_ranges);

    // Convert back to LineSpec
    merged
        .into_iter()
        .map(|(start, end)| {
            if start == end {
                LineSpec::Single(start)
            } else {
                LineSpec::Range(start, end)
            }
        })
        .collect()
}

impl Selector {
    /// Parse a selector string.
    ///
    /// # Examples
    /// ```
    /// use sel::selector::Selector;
    ///
    /// let sel = Selector::parse("42").unwrap();
    /// let sel = Selector::parse("10-20").unwrap();
    /// let sel = Selector::parse("1,5,10-15").unwrap();
    /// let sel = Selector::parse("23:260").unwrap();
    /// ```
    pub fn parse(s: &str) -> Result<Self> {
        if s.is_empty() {
            return Ok(Selector::All);
        }

        // Check if any element contains ':' (positional selector)
        let has_position = s.contains(':');

        let parts: Vec<&str> = s.split(',').collect();

        if has_position {
            // All elements must be positional
            let mut positions = Vec::new();
            for part in parts {
                let pos = Position::parse(part)?;
                positions.push(pos);
            }
            Ok(Selector::Positions(positions))
        } else {
            // All elements are line numbers or ranges
            let mut specs = Vec::new();
            for part in parts {
                let spec = LineSpec::parse(part)?;
                specs.push(spec);
            }
            Ok(Selector::LineNumbers(specs))
        }
    }

    /// Normalize the selector by merging adjacent/range specs.
    pub fn normalize(&self) -> Self {
        match self {
            Selector::All => Selector::All,
            Selector::Positions(p) => {
                // Remove duplicates and sort
                let unique: BTreeSet<_> = p.iter().collect();
                Selector::Positions(unique.into_iter().cloned().collect())
            }
            Selector::LineNumbers(specs) => {
                // Sort and merge overlapping ranges
                let mut lines: Vec<usize> = Vec::new();
                let mut ranges: Vec<(usize, usize)> = Vec::new();

                for spec in specs {
                    match spec {
                        LineSpec::Single(n) => lines.push(*n),
                        LineSpec::Range(start, end) => ranges.push((*start, *end)),
                    }
                }

                lines.sort();
                lines.dedup();
                ranges.sort();

                // Merge overlapping ranges
                let merged_ranges = merge_ranges(ranges);

                // Convert single lines to ranges and merge with existing ranges
                let result = merge_lines_with_ranges(lines, merged_ranges);

                Selector::LineNumbers(result)
            }
        }
    }

    /// Returns true if this is a positional selector.
    pub fn is_positional(&self) -> bool {
        matches!(self, Selector::Positions(_))
    }
}

/// A line specification: either a single line or a range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineSpec {
    /// Single line number.
    Single(usize),

    /// Range of lines (inclusive).
    Range(usize, usize),
}

impl LineSpec {
    /// Parse a line spec string.
    ///
    /// # Examples
    /// ```
    /// # use sel::selector::LineSpec;
    /// let spec = LineSpec::parse("42").unwrap();
    /// let spec = LineSpec::parse("10-20").unwrap();
    /// ```
    pub fn parse(s: &str) -> Result<Self> {
        if let Some((start, end)) = s.split_once('-') {
            let start = start.parse::<usize>().map_err(|_| {
                SelError::InvalidSelector(format!("Invalid range start: '{}'", start))
            })?;
            let end = end
                .parse::<usize>()
                .map_err(|_| SelError::InvalidSelector(format!("Invalid range end: '{}'", end)))?;

            if start == 0 || end == 0 {
                return Err(SelError::InvalidSelector(
                    "Line numbers must be >= 1".to_string(),
                ));
            }

            if start > end {
                return Err(SelError::InvalidSelector(format!(
                    "Range start ({}) > end ({})",
                    start, end
                )));
            }

            Ok(LineSpec::Range(start, end))
        } else {
            let n = s
                .parse::<usize>()
                .map_err(|_| SelError::InvalidSelector(format!("Invalid line number: '{}'", s)))?;

            if n == 0 {
                return Err(SelError::InvalidSelector(
                    "Line number must be >= 1".to_string(),
                ));
            }

            Ok(LineSpec::Single(n))
        }
    }

    /// Check if this spec contains the given line number.
    pub fn contains(&self, line: usize) -> bool {
        match self {
            LineSpec::Single(n) => *n == line,
            LineSpec::Range(start, end) => line >= *start && line <= *end,
        }
    }

    /// Get the starting line number.
    pub fn start(&self) -> usize {
        match self {
            LineSpec::Single(n) => *n,
            LineSpec::Range(start, _) => *start,
        }
    }
}

/// A position in a file (line:column).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    /// Line number (1-indexed).
    pub line: usize,

    /// Column number in bytes (1-indexed).
    pub column: usize,
}

impl Position {
    /// Create a new position.
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }

    /// Parse a position string.
    ///
    /// # Examples
    /// ```
    /// # use sel::selector::Position;
    /// let pos = Position::parse("23:260").unwrap();
    /// assert_eq!(pos.line, 23);
    /// assert_eq!(pos.column, 260);
    /// ```
    pub fn parse(s: &str) -> Result<Self> {
        let (line_str, col_str) = s.split_once(':').ok_or_else(|| {
            SelError::InvalidSelector(format!("Invalid position format: '{}'", s))
        })?;

        let line = line_str.parse::<usize>().map_err(|_| {
            SelError::InvalidSelector(format!("Invalid line number in position: '{}'", line_str))
        })?;

        let column = col_str.parse::<usize>().map_err(|_| {
            SelError::InvalidSelector(format!("Invalid column number in position: '{}'", col_str))
        })?;

        if line == 0 || column == 0 {
            return Err(SelError::InvalidSelector(
                "Line and column numbers must be >= 1".to_string(),
            ));
        }

        Ok(Position { line, column })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_line() {
        let sel = Selector::parse("42").unwrap();
        assert_eq!(sel, Selector::LineNumbers(vec![LineSpec::Single(42)]));
    }

    #[test]
    fn parse_range() {
        let sel = Selector::parse("10-20").unwrap();
        assert_eq!(sel, Selector::LineNumbers(vec![LineSpec::Range(10, 20)]));
    }

    #[test]
    fn parse_multiple_specs() {
        let sel = Selector::parse("1,5,10-15,20").unwrap();
        assert_eq!(
            sel,
            Selector::LineNumbers(vec![
                LineSpec::Single(1),
                LineSpec::Single(5),
                LineSpec::Range(10, 15),
                LineSpec::Single(20),
            ])
        );
    }

    #[test]
    fn parse_position() {
        let sel = Selector::parse("23:260").unwrap();
        assert_eq!(sel, Selector::Positions(vec![Position::new(23, 260)]));
    }

    #[test]
    fn parse_multiple_positions() {
        let sel = Selector::parse("15:30,23:260").unwrap();
        assert_eq!(
            sel,
            Selector::Positions(vec![Position::new(15, 30), Position::new(23, 260),])
        );
    }

    #[test]
    fn reject_mixed_selector() {
        // This test verifies that mixing positional and non-positional is rejected
        // However, our current implementation checks for ':' globally, so "1,23:260"
        // would be treated as positional and fail when parsing "1"
        let result = Selector::parse("1,23:260");
        assert!(result.is_err());
    }

    #[test]
    fn parse_empty_selector() {
        let sel = Selector::parse("").unwrap();
        assert_eq!(sel, Selector::All);
    }

    #[test]
    fn reject_zero_line() {
        assert!(Selector::parse("0").is_err());
        assert!(Selector::parse("10-0").is_err());
    }

    #[test]
    fn reject_invalid_range() {
        assert!(Selector::parse("20-10").is_err());
    }

    #[test]
    fn line_spec_contains() {
        let spec = LineSpec::Range(10, 20);
        assert!(!spec.contains(9));
        assert!(spec.contains(10));
        assert!(spec.contains(15));
        assert!(spec.contains(20));
        assert!(!spec.contains(21));
    }

    #[test]
    fn normalize_merges_overlapping_ranges() {
        let sel = Selector::LineNumbers(vec![LineSpec::Range(1, 5), LineSpec::Range(3, 10)]);
        let normalized = sel.normalize();
        assert_eq!(
            normalized,
            Selector::LineNumbers(vec![LineSpec::Range(1, 10)])
        );
    }

    #[test]
    fn normalize_merges_adjacent_ranges() {
        let sel = Selector::LineNumbers(vec![LineSpec::Range(1, 5), LineSpec::Range(6, 10)]);
        let normalized = sel.normalize();
        assert_eq!(
            normalized,
            Selector::LineNumbers(vec![LineSpec::Range(1, 10)])
        );
    }

    #[test]
    fn normalize_merges_single_lines_into_ranges() {
        let sel = Selector::LineNumbers(vec![
            LineSpec::Single(1),
            LineSpec::Single(2),
            LineSpec::Single(3),
            LineSpec::Single(10),
        ]);
        let normalized = sel.normalize();
        assert_eq!(
            normalized,
            Selector::LineNumbers(vec![LineSpec::Range(1, 3), LineSpec::Single(10)])
        );
    }

    #[test]
    fn normalize_complex_merge() {
        let sel = Selector::LineNumbers(vec![
            LineSpec::Range(1, 5),
            LineSpec::Single(6),
            LineSpec::Range(7, 10),
            LineSpec::Range(15, 20),
            LineSpec::Single(21),
        ]);
        let normalized = sel.normalize();
        assert_eq!(
            normalized,
            Selector::LineNumbers(vec![LineSpec::Range(1, 10), LineSpec::Range(15, 21)])
        );
    }

    #[test]
    fn normalize_keeps_non_adjacent_ranges_separate() {
        let sel = Selector::LineNumbers(vec![LineSpec::Range(1, 5), LineSpec::Range(10, 15)]);
        let normalized = sel.normalize();
        assert_eq!(
            normalized,
            Selector::LineNumbers(vec![LineSpec::Range(1, 5), LineSpec::Range(10, 15)])
        );
    }

    #[test]
    fn normalize_removes_duplicate_lines() {
        let sel = Selector::LineNumbers(vec![
            LineSpec::Single(5),
            LineSpec::Single(5),
            LineSpec::Single(5),
        ]);
        let normalized = sel.normalize();
        assert_eq!(normalized, Selector::LineNumbers(vec![LineSpec::Single(5)]));
    }

    #[test]
    fn normalize_empty_selector() {
        let sel = Selector::LineNumbers(vec![]);
        let normalized = sel.normalize();
        assert_eq!(normalized, Selector::LineNumbers(vec![]));
    }
}
