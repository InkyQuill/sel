//! Output formatting and coloring.

use crate::cli::ColorMode;
use std::io::{self, Write};
use std::ops::Range;

/// Output formatter for `sel`.
pub struct OutputFormatter<W: Write> {
    writer: W,
    show_line_numbers: bool,
    show_filename: bool,
    filename: Option<String>,
    color_mode: ColorMode,
}

impl<W: Write> OutputFormatter<W> {
    /// Create a new output formatter.
    pub fn new(
        writer: W,
        show_line_numbers: bool,
        show_filename: bool,
        filename: Option<String>,
        color_mode: ColorMode,
    ) -> Self {
        Self {
            writer,
            show_line_numbers,
            show_filename,
            filename,
            color_mode,
        }
    }

    /// Write a line with optional line number and filename.
    pub fn write_line(&mut self, line_no: usize, content: &str) -> io::Result<()> {
        let prefix = self.format_prefix(line_no);
        writeln!(self.writer, "{}{}", prefix, content)
    }

    /// Write a target line (with marker).
    pub fn write_target_line(&mut self, line_no: usize, content: &str) -> io::Result<()> {
        let prefix = self.format_prefix(line_no);
        let marker = if self.color_mode.should_colorize() {
            // ANSI green color for target line marker
            "\x1b[32m>\x1b[0m "
        } else {
            "> "
        };
        writeln!(self.writer, "{}{}{}", marker, prefix, content)
    }

    /// Write a context line (non-target).
    pub fn write_context_line(&mut self, line_no: usize, content: &str) -> io::Result<()> {
        self.write_line(line_no, content)
    }

    /// Write a fragment with character context and pointer.
    pub fn write_fragment(
        &mut self,
        line_no: usize,
        fragment: &str,
        pointer_offset: usize,
    ) -> io::Result<()> {
        // Write the fragment line
        if self.show_line_numbers {
            writeln!(self.writer, "{}:{}", line_no, fragment)?;
        } else {
            writeln!(self.writer, "{}", fragment)?;
        }

        // Write the pointer line
        let spaces = " ".repeat(pointer_offset);
        let pointer = if self.color_mode.should_colorize() {
            // ANSI color for pointer
            format!("\x1b[32m{}^\x1b[0m", spaces)
        } else {
            format!("{}^", spaces)
        };
        writeln!(self.writer, "{}", pointer)?;

        Ok(())
    }

    /// Write a line with highlighted matches.
    ///
    /// Matches are specified as byte ranges within the line.
    pub fn write_line_with_matches(
        &mut self,
        line_no: usize,
        content: &str,
        matches: &[Range<usize>],
    ) -> io::Result<()> {
        let prefix = self.format_prefix(line_no);

        if self.color_mode.should_colorize() && !matches.is_empty() {
            // Highlight matches using ANSI escape codes
            let highlighted = self.highlight_matches(content, matches);
            writeln!(self.writer, "{}{}", prefix, highlighted)
        } else {
            writeln!(self.writer, "{}{}", prefix, content)
        }
    }

    /// Write a target line with marker and highlighted matches.
    pub fn write_target_line_with_matches(
        &mut self,
        line_no: usize,
        content: &str,
        matches: &[Range<usize>],
    ) -> io::Result<()> {
        let prefix = self.format_prefix(line_no);
        let marker = if self.color_mode.should_colorize() {
            "\x1b[32m>\x1b[0m "
        } else {
            "> "
        };

        if self.color_mode.should_colorize() && !matches.is_empty() {
            let highlighted = self.highlight_matches(content, matches);
            writeln!(self.writer, "{}{}{}", marker, prefix, highlighted)
        } else {
            writeln!(self.writer, "{}{}{}", marker, prefix, content)
        }
    }

    /// Write a fragment with character context and highlighted match.
    pub fn write_fragment_with_match(
        &mut self,
        line_no: usize,
        fragment: &str,
        match_range_in_fragment: Range<usize>,
    ) -> io::Result<()> {
        // Write the fragment line with highlight
        if self.show_line_numbers {
            write!(self.writer, "{}:", line_no)?;
        }

        if self.color_mode.should_colorize() {
            let highlighted = self.highlight_substring(fragment, &match_range_in_fragment);
            writeln!(self.writer, "{}", highlighted)?;
        } else {
            writeln!(self.writer, "{}", fragment)?;
        }

        // Write the pointer line
        let spaces = " ".repeat(match_range_in_fragment.start);
        let pointer = if self.color_mode.should_colorize() {
            format!("\x1b[32m{}^\x1b[0m", spaces)
        } else {
            format!("{}^", spaces)
        };
        writeln!(self.writer, "{}", pointer)?;

        Ok(())
    }

    /// Highlight all matches in a string.
    fn highlight_matches(&self, text: &str, matches: &[Range<usize>]) -> String {
        let mut result = String::new();
        let mut last_end = 0;

        // Sort matches by start position
        let mut sorted_matches = matches.to_vec();
        sorted_matches.sort_by_key(|m| m.start);

        for m in sorted_matches {
            // Add text before the match
            if m.start > last_end {
                result.push_str(&text[last_end..m.start]);
            }

            // Add the highlighted match
            let match_text = &text[m.start..m.end.min(text.len())];
            result.push_str("\x1b[7m"); // Inverse video
            result.push_str(match_text);
            result.push_str("\x1b[0m"); // Reset

            last_end = m.end.max(last_end);
        }

        // Add remaining text
        if last_end < text.len() {
            result.push_str(&text[last_end..]);
        }

        result
    }

    /// Highlight a substring within a larger string.
    fn highlight_substring(&self, text: &str, range: &Range<usize>) -> String {
        let start = range.start.min(text.len());
        let end = range.end.min(text.len());

        if start >= end {
            return text.to_string();
        }

        let before = &text[..start];
        let matched = &text[start..end];
        let after = &text[end..];

        format!("{}\x1b[7m{}\x1b[0m{}", before, matched, after)
    }

    /// Flush any pending output.
    pub fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }

    /// Format the prefix (filename and line number) for a line.
    fn format_prefix(&self, line_no: usize) -> String {
        let mut prefix = String::new();

        if self.show_filename {
            if let Some(filename) = &self.filename {
                prefix.push_str(filename);
                prefix.push(':');
            }
        }

        if self.show_line_numbers {
            prefix.push_str(&line_no.to_string());
            prefix.push(':');
        }

        prefix
    }
}

/// A fragment of a line with character context.
#[derive(Debug, Clone)]
pub struct Fragment {
    pub line_number: usize,
    pub content: String,
    pub start_column: usize,
    pub target_column: usize,
}

impl Fragment {
    /// Create a new fragment from a line and position with character context.
    pub fn new(line: &str, column: usize, context: usize) -> Self {
        let line_bytes = line.as_bytes();
        let line_len = line_bytes.len();

        // Column is 1-indexed, convert to 0-indexed
        // Clamp to line length to handle positions beyond end of line
        let col_idx = column.saturating_sub(1).min(line_len.saturating_sub(1));

        // Calculate fragment bounds
        // Ensure start is within bounds
        let start = if col_idx <= context {
            0
        } else {
            col_idx - context
        };
        let start = std::cmp::min(start, line_len);
        let end = std::cmp::min(line_len, col_idx + context + 1);

        // Extract fragment (assuming valid UTF-8 for simplicity)
        let content = if start < end {
            String::from_utf8_lossy(&line_bytes[start..end]).to_string()
        } else {
            String::new()
        };

        Fragment {
            line_number: 0, // Will be set by caller
            content,
            start_column: start + 1, // Convert back to 1-indexed
            target_column: column,
        }
    }

    /// Get the pointer offset (number of spaces before the `^` marker).
    pub fn pointer_offset(&self) -> usize {
        // The pointer should be under the target column
        // Relative to the start of the fragment
        self.target_column.saturating_sub(self.start_column)
    }

    /// Format as a display string.
    pub fn format(&self) -> String {
        format!("{}:{}", self.line_number, self.content)
    }

    /// Format the pointer line.
    pub fn format_pointer(&self) -> String {
        let spaces = " ".repeat(self.pointer_offset());
        format!("{}^", spaces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fragment_middle() {
        let line = "This is a test line with some content";
        let frag = Fragment::new(line, 11, 5); // Position 11 (the 'a' in 'a test')
        assert!(frag.content.contains("a test"));
    }

    #[test]
    fn test_fragment_start() {
        let line = "This is a test line";
        let frag = Fragment::new(line, 1, 5);
        assert_eq!(frag.start_column, 1);
        assert!(frag.content.starts_with("This"));
    }

    #[test]
    fn test_fragment_end() {
        let line = "This is a test line";
        let frag = Fragment::new(line, 100, 5); // Beyond end
        // Should show the end of the line with context
        assert!(frag.content.ends_with("line"));
        assert!(frag.content.len() <= line.len());
    }

    #[test]
    fn test_fragment_pointer_offset() {
        let line = "This is a test line";
        let frag = Fragment::new(line, 11, 5);
        // Position 11 is within the fragment
        assert_eq!(frag.pointer_offset(), 11 - frag.start_column);
    }

    #[test]
    fn test_fragment_short_line() {
        let line = "abc";
        let frag = Fragment::new(line, 2, 10);
        assert_eq!(frag.content, "abc");
    }

    #[test]
    fn test_highlight_matches_single() {
        let formatter = create_test_formatter(ColorMode::Always);
        let text = "Hello ERROR in this line";
        let matches = vec![6..11]; // "ERROR"
        let result = formatter.highlight_matches(text, &matches);
        // Should contain ANSI escape codes for inverse video
        assert!(result.contains("\x1b[7m"));
        assert!(result.contains("\x1b[0m"));
        assert!(result.contains("ERROR"));
    }

    #[test]
    fn test_highlight_matches_multiple() {
        let formatter = create_test_formatter(ColorMode::Always);
        let text = "ERROR: something went wrong with ERROR code";
        let matches = vec![0..5, 33..38]; // Two "ERROR"s
        let result = formatter.highlight_matches(text, &matches);
        // Should highlight both matches
        assert!(result.contains("\x1b[7mERROR\x1b[0m"));
        // Count how many times we have the highlight start marker
        let count = result.matches("\x1b[7m").count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_highlight_matches_no_color() {
        let formatter = create_test_formatter(ColorMode::Never);
        let text = "Hello ERROR in this line";
        let matches = vec![6..11];
        let result = formatter.highlight_matches(text, &matches);
        // Even with ColorMode::Never, highlight_matches still adds colors
        // The decision to use it is made by the caller
        assert!(result.contains("\x1b[7m"));
    }

    #[test]
    fn test_highlight_substring() {
        let formatter = create_test_formatter(ColorMode::Always);
        let text = "This is a test line";
        let range = 5..7; // "is"
        let result = formatter.highlight_substring(text, &range);
        assert_eq!(result, "This \x1b[7mis\x1b[0m a test line");
    }

    #[test]
    fn test_highlight_substring_at_start() {
        let formatter = create_test_formatter(ColorMode::Always);
        let text = "ERROR in this line";
        let range = 0..5;
        let result = formatter.highlight_substring(text, &range);
        assert_eq!(result, "\x1b[7mERROR\x1b[0m in this line");
    }

    #[test]
    fn test_highlight_substring_at_end() {
        let formatter = create_test_formatter(ColorMode::Always);
        let text = "This line has ERROR";
        let range = 14..19;
        let result = formatter.highlight_substring(text, &range);
        assert_eq!(result, "This line has \x1b[7mERROR\x1b[0m");
    }

    #[test]
    fn test_highlight_substring_empty_range() {
        let formatter = create_test_formatter(ColorMode::Always);
        let text = "This is a test";
        let range = 5..5; // Empty range
        let result = formatter.highlight_substring(text, &range);
        // Should return original text
        assert_eq!(result, text);
    }

    #[test]
    fn test_highlight_matches_overlapping() {
        let formatter = create_test_formatter(ColorMode::Always);
        let text = "ERROR123ERROR";
        let matches = vec![0..5, 5..10]; // Overlapping at position 5
        let result = formatter.highlight_matches(text, &matches);
        // Should handle overlapping ranges gracefully
        assert!(result.contains("\x1b[7m"));
    }

    #[test]
    fn test_highlight_matches_out_of_bounds() {
        let formatter = create_test_formatter(ColorMode::Always);
        let text = "Short";
        let matches = vec![0..100]; // Beyond text length
        let result = formatter.highlight_matches(text, &matches);
        // Should clamp to text length
        assert!(result.contains("\x1b[7mShort\x1b[0m"));
    }

    /// Helper to create a test formatter.
    fn create_test_formatter(color_mode: ColorMode) -> OutputFormatter<Vec<u8>> {
        OutputFormatter::new(
            Vec::new(),
            false,
            false,
            None,
            color_mode,
        )
    }
}
