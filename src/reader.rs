//! Streaming file reader with line numbering.

use crate::error::Result;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

/// A reader that yields lines with their line numbers (1-indexed).
pub struct LineReader<R: Read> {
    reader: BufReader<R>,
    current_line: usize,
}

impl<R: Read> LineReader<R> {
    /// Create a new line reader.
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
            current_line: 0,
        }
    }

    /// Read the next line, returning the line number and content.
    ///
    /// Returns `Ok(None)` at EOF.
    pub fn read_line(&mut self) -> Result<Option<(usize, String)>> {
        let mut line = String::new();
        let bytes_read = self.reader.read_line(&mut line)?;

        if bytes_read == 0 {
            return Ok(None);
        }

        self.current_line += 1;

        // Remove trailing newline(s) while preserving other whitespace
        while line.ends_with('\n') || line.ends_with('\r') {
            line.pop();
        }

        Ok(Some((self.current_line, line)))
    }

    /// Get the current line number.
    pub fn current_line(&self) -> usize {
        self.current_line
    }

    /// Reset the line counter (useful when processing multiple files).
    pub fn reset_line_counter(&mut self) {
        self.current_line = 0;
    }
}

/// Open a file for reading.
pub fn open_file(path: &Path) -> Result<std::fs::File> {
    std::fs::File::open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            crate::error::SelError::FileNotFound(path.to_path_buf())
        } else {
            crate::error::SelError::from(e)
        }
    })
}

/// A context buffer for storing lines before a match.
///
/// This is used for implementing the `-c` context option.
pub struct ContextBuffer {
    buffer: Vec<Option<(usize, String)>>,
    capacity: usize,
    pos: usize,  // Current position in the circular buffer
    size: usize, // Number of items currently in buffer
}

impl ContextBuffer {
    /// Create a new context buffer with the given capacity.
    ///
    /// The buffer stores up to `capacity` lines before the current line.
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![None; capacity],
            capacity,
            pos: 0,
            size: 0,
        }
    }

    /// Add a line to the buffer.
    pub fn push(&mut self, line_no: usize, line: String) {
        if self.capacity == 0 {
            return;
        }

        let was_empty = self.buffer[self.pos].is_none();
        self.buffer[self.pos] = Some((line_no, line));
        self.pos = (self.pos + 1) % self.capacity;

        if was_empty {
            self.size = self.size.saturating_add(1);
        }
    }

    /// Get the stored lines in order (oldest to newest).
    pub fn get_lines(&self) -> Vec<(usize, String)> {
        let mut result = Vec::with_capacity(self.capacity);

        if self.capacity == 0 {
            return result;
        }

        // Start from the oldest entry
        let start = if self.size == self.capacity {
            self.pos // Buffer has wrapped around
        } else {
            0 // Buffer hasn't wrapped
        };

        for i in 0..self.size {
            let idx = (start + i) % self.capacity;
            if let Some(entry) = &self.buffer[idx] {
                result.push(entry.clone());
            }
        }

        result
    }

    /// Get the number of lines currently stored.
    pub fn len(&self) -> usize {
        self.size
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Clear the buffer.
    pub fn clear(&mut self) {
        self.buffer = vec![None; self.capacity];
        self.pos = 0;
        self.size = 0;
    }

    /// Drain the buffer, returning all stored lines and clearing it.
    pub fn drain(&mut self) -> Vec<(usize, String)> {
        let result = self.get_lines();
        self.clear();
        result
    }
}

/// A context range representing a span of lines to output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextRange {
    pub start: usize,
    pub end: usize,
}

impl ContextRange {
    /// Create a new context range.
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Create a context range centered around a target line.
    pub fn around(target: usize, context: usize) -> Self {
        let start = target.saturating_sub(context);
        let end = target.saturating_add(context);
        Self { start, end }
    }

    /// Check if this range overlaps with another.
    pub fn overlaps(&self, other: &Self) -> bool {
        // Ranges overlap if one starts before or at the other's end
        // and ends at or after the other's start
        self.start <= other.end + 1 && other.start <= self.end + 1
    }

    /// Merge this range with another (must be overlapping or adjacent).
    pub fn merge(&mut self, other: &Self) {
        self.start = self.start.min(other.start);
        self.end = self.end.max(other.end);
    }
}

/// Merge potentially overlapping context ranges into consolidated ranges.
pub fn merge_context_ranges(ranges: Vec<ContextRange>) -> Vec<ContextRange> {
    if ranges.is_empty() {
        return Vec::new();
    }

    let mut sorted: Vec<_> = ranges.into_iter().collect();
    sorted.sort_by_key(|r| r.start);

    let mut result = Vec::new();
    let mut current = sorted[0].clone();

    for range in &sorted[1..] {
        if current.overlaps(range) {
            current.merge(range);
        } else {
            result.push(current);
            current = range.clone();
        }
    }
    result.push(current);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_reader() {
        let data = b"line1\nline2\nline3";
        let mut reader = LineReader::new(&data[..]);

        assert_eq!(reader.read_line().unwrap(), Some((1, "line1".to_string())));
        assert_eq!(reader.read_line().unwrap(), Some((2, "line2".to_string())));
        assert_eq!(reader.read_line().unwrap(), Some((3, "line3".to_string())));
        assert_eq!(reader.read_line().unwrap(), None);
    }

    #[test]
    fn test_context_buffer() {
        let mut buf = ContextBuffer::new(3);

        buf.push(1, "a".to_string());
        buf.push(2, "b".to_string());
        buf.push(3, "c".to_string());

        let lines = buf.get_lines();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].0, 1);
        assert_eq!(lines[1].0, 2);
        assert_eq!(lines[2].0, 3);
    }

    #[test]
    fn test_context_buffer_wraparound() {
        let mut buf = ContextBuffer::new(2);

        buf.push(1, "a".to_string());
        buf.push(2, "b".to_string());
        buf.push(3, "c".to_string()); // Overwrites "a"

        let lines = buf.get_lines();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].0, 2);
        assert_eq!(lines[1].0, 3);
    }

    #[test]
    fn test_context_buffer_drain() {
        let mut buf = ContextBuffer::new(3);

        buf.push(1, "a".to_string());
        buf.push(2, "b".to_string());

        let lines = buf.drain();
        assert_eq!(lines.len(), 2);
        assert_eq!(buf.len(), 0);
        assert!(buf.is_empty());
    }

    #[test]
    fn test_context_range_around() {
        let range = ContextRange::around(5, 2);
        assert_eq!(range.start, 3);
        assert_eq!(range.end, 7);
    }

    #[test]
    fn test_context_range_near_start() {
        let range = ContextRange::around(2, 3);
        assert_eq!(range.start, 0); // Saturates to 0
        assert_eq!(range.end, 5);
    }

    #[test]
    fn test_context_range_overlaps() {
        let r1 = ContextRange::new(1, 5);
        let r2 = ContextRange::new(3, 8);

        assert!(r1.overlaps(&r2));
        assert!(r2.overlaps(&r1));
    }

    #[test]
    fn test_context_range_adjacent() {
        let r1 = ContextRange::new(1, 5);
        let r2 = ContextRange::new(6, 10);

        // Adjacent ranges should overlap (5+1 >= 6)
        assert!(r1.overlaps(&r2));
    }

    #[test]
    fn test_context_range_no_overlap() {
        let r1 = ContextRange::new(1, 5);
        let r2 = ContextRange::new(7, 10);

        assert!(!r1.overlaps(&r2));
    }

    #[test]
    fn test_merge_context_ranges() {
        let ranges = vec![
            ContextRange::new(1, 3),
            ContextRange::new(5, 7),
            ContextRange::new(6, 9), // Overlaps with (5, 7)
        ];

        let merged = merge_context_ranges(ranges);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0], ContextRange::new(1, 3));
        assert_eq!(merged[1], ContextRange::new(5, 9));
    }

    #[test]
    fn test_merge_context_ranges_all_overlapping() {
        let ranges = vec![
            ContextRange::new(1, 5),
            ContextRange::new(3, 7),
            ContextRange::new(6, 10),
        ];

        let merged = merge_context_ranges(ranges);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0], ContextRange::new(1, 10));
    }
}
