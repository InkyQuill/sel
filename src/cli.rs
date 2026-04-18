//! Command-line argument parsing using clap.

use clap::Parser;
use std::io::IsTerminal;
use std::path::PathBuf;

/// `sel` — Select slices from text files by line numbers, ranges, positions, or regex.
#[derive(Parser, Debug)]
#[command(name = "sel")]
#[command(author = "InkyQuill")]
#[command(version = "0.1.0")]
#[command(about = "Select slices from text files", long_about = None)]
#[command(
    long_about = "Extract fragments from text files by line numbers, ranges, positions (line:column), or regex patterns.

EXAMPLES:
    sel 30-35 file.txt           Output lines 30-35
    sel 10,15-20,22 file.txt     Output lines 10, 15-20, and 22
    sel -c 3 42 file.txt         Show line 42 with 3 lines of context
    sel -n 10 23:260 file.txt    Show position line 23, column 260 with char context
    sel -e ERROR log.txt         Search for 'ERROR' pattern
    sel file.txt                 Output entire file with line numbers (like cat -n)"
)]
pub struct Cli {
    /// Show N lines of context before and after matches
    #[arg(short = 'c', long = "context", value_name = "N")]
    pub context: Option<usize>,

    /// Show N characters of context around position
    ///
    /// Only works with positional selectors (L:C) or with -e.
    #[arg(short = 'n', long = "char-context", value_name = "N")]
    pub char_context: Option<usize>,

    /// Don't output line numbers
    ///
    /// Filenames are still shown when processing multiple files.
    #[arg(short = 'l', long = "no-line-numbers")]
    pub no_line_numbers: bool,

    /// Regular expression pattern (PCRE-like syntax)
    ///
    /// When using -e, the selector argument is ignored.
    /// Multiple files can be specified with -e.
    #[arg(short = 'e', long = "regex", value_name = "PATTERN")]
    pub regex: Option<String>,

    /// Invert the regex match: emit lines that do NOT match -e.
    #[arg(short = 'v', long = "invert-match")]
    pub invert: bool,

    /// Always print filename prefix
    ///
    /// By default, filename is only shown when processing multiple files.
    #[arg(short = 'H', long = "with-filename")]
    pub with_filename: bool,

    /// Color output [auto, always, never]
    ///
    /// Default is 'auto' (enabled when stdout is a terminal).
    #[arg(long = "color", value_name = "WHEN")]
    pub color: Option<String>,

    /// Write output to FILE instead of stdout. Use `-` for stdout explicitly.
    #[arg(short = 'o', long = "output", value_name = "FILE")]
    pub output: Option<String>,

    /// With `-o`, overwrite an existing file.
    #[arg(long = "force")]
    pub force: bool,

    /// Selector and/or file(s)
    ///
    /// The first positional argument can be:
    /// - A selector (line number, range, position) if it matches selector syntax
    /// - A filename otherwise
    ///
    /// When using -e, all positional arguments are treated as files.
    #[arg(value_name = "SELECTOR_OR_FILE")]
    pub args: Vec<String>,
}

impl Cli {
    /// Get the selector from arguments (only valid when not using -e).
    pub fn get_selector(&self) -> Option<String> {
        if self.regex.is_some() {
            return None;
        }

        if self.args.is_empty() {
            return None;
        }

        // Check if first arg looks like a selector
        let first = &self.args[0];

        // A selector is:
        // - A single number (e.g., "42")
        // - A range (e.g., "10-20")
        // - A comma-separated list (e.g., "1,5,10-15")
        // - A position (e.g., "23:260")
        // - Contains only digits, commas, colons, and hyphens
        if self.looks_like_selector(first) {
            Some(first.clone())
        } else {
            None
        }
    }

    /// Get the list of input files.
    ///
    /// Returns at least one entry — falls back to `-` (stdin) when no
    /// explicit files are provided.
    pub fn get_files(&self) -> Vec<PathBuf> {
        if self.args.is_empty() {
            return vec![PathBuf::from("-")];
        }

        // If using regex mode, all args are files
        if self.regex.is_some() {
            return self.args.iter().map(PathBuf::from).collect();
        }

        // If first arg is a selector, skip it
        let start = if self.looks_like_selector(&self.args[0]) {
            1
        } else {
            0
        };

        let files: Vec<_> = self.args[start..].iter().map(PathBuf::from).collect();
        if files.is_empty() {
            vec![PathBuf::from("-")]
        } else {
            files
        }
    }

    /// Check if a string looks like a selector.
    fn looks_like_selector(&self, s: &str) -> bool {
        // `-` is the stdin sentinel, not a selector.
        if s == "-" {
            return false;
        }
        // Empty string is not a selector
        if s.is_empty() {
            return false;
        }

        // Check if it's a valid selector pattern
        // Contains only: digits, commas, colons, hyphens
        // And at least one digit
        let has_digit = s.chars().any(|c| c.is_ascii_digit());
        if !has_digit {
            return false;
        }

        // Check for invalid characters
        let valid_chars = s
            .chars()
            .all(|c| c.is_ascii_digit() || c == ',' || c == ':' || c == '-');

        if !valid_chars {
            return false;
        }

        // Additional validation: colons must be between numbers
        // e.g., "23:260" is valid, but ":260" or "23:" is not
        if s.contains(':') {
            for part in s.split(',') {
                if let Some((line, col)) = part.split_once(':') {
                    // Both sides must be non-empty numbers
                    if line.is_empty() || col.is_empty() {
                        return false;
                    }
                    if !line.chars().all(|c| c.is_ascii_digit()) {
                        return false;
                    }
                    if !col.chars().all(|c| c.is_ascii_digit()) {
                        return false;
                    }
                }
            }
        }

        true
    }

    /// Validate CLI arguments and check for conflicts.
    pub fn validate(&self) -> crate::Result<()> {
        if self.invert && self.regex.is_none() {
            return Err(crate::SelError::InvertWithoutRegex);
        }
        if self.char_context.is_some()
            && self.regex.is_none()
            && !self
                .get_selector()
                .as_ref()
                .is_some_and(|s| s.contains(':'))
        {
            return Err(crate::SelError::CharContextWithoutTarget);
        }

        Ok(())
    }

    /// Get the color mode based on the --color flag and terminal detection.
    pub fn color_mode(&self) -> ColorMode {
        match self.color.as_deref() {
            Some("always") => ColorMode::Always,
            Some("never") => ColorMode::Never,
            Some("auto") | None => {
                // Check if stdout is a terminal
                if std::io::stdout().is_terminal() {
                    ColorMode::Always
                } else {
                    ColorMode::Never
                }
            }
            Some(_) => ColorMode::Never, // Invalid value, default to never
        }
    }
}

use crate::app::{NonSeek, Seek, Stage1};
use crate::context::{LineContext, NoContext};
use crate::format::{FormatOpts, FragmentFormatter, PlainFormatter};
use crate::matcher::{AllMatcher, LineMatcher, PositionMatcher, RegexMatcher};
use crate::sink::{FileSink, StdoutSink};
use crate::source::{FileSource, Source, StdinSource};
use crate::{App, Selector};

impl Cli {
    /// Construct the output sink based on `--output`/`--force` flags.
    fn make_sink(&self) -> crate::Result<Box<dyn crate::sink::Sink>> {
        match self.output.as_deref() {
            None | Some("-") => Ok(Box::new(StdoutSink::new())),
            Some(path) => {
                let sink = FileSink::create(std::path::Path::new(path), self.force)?;
                Ok(Box::new(sink))
            }
        }
    }

    /// Resolve `--color` against whether the sink is a terminal.
    fn resolve_color(&self, to_terminal: bool) -> bool {
        match self.color.as_deref() {
            Some("always") => true,
            Some("never") => false,
            _ => to_terminal,
        }
    }

    /// Build a ready-to-run `App` for a single file.
    ///
    /// Callers iterate over `get_files()` and build one `App` per file.
    pub fn into_app_for_file(
        &self,
        path: &std::path::Path,
        show_filename: bool,
    ) -> crate::Result<App<Seek>> {
        let source = FileSource::open(path)?;
        let filename = if show_filename {
            Some(source.label().to_string())
        } else {
            None
        };
        let sink = self.make_sink()?;
        let color = self.resolve_color(sink.is_terminal());
        let opts = FormatOpts {
            show_line_numbers: !self.no_line_numbers,
            show_filename,
            filename,
            color,
            // Target marker (`> `) only appears in context-aware output.
            target_marker: matches!(self.context, Some(n) if n > 0),
        };

        // Matcher + seek stage.
        let stage2 = Stage1::with_seekable_source(Box::new(source));
        let stage3 = if let Some(pat) = &self.regex {
            stage2.with_matcher(Box::new(RegexMatcher::new(pat, self.invert)?))
        } else if let Some(raw) = self.get_selector() {
            let sel = Selector::parse(&raw)?;
            match sel {
                Selector::All => stage2.with_matcher(Box::new(AllMatcher)),
                Selector::LineNumbers(_) => {
                    stage2.with_matcher(Box::new(LineMatcher::from_selector(&sel)))
                }
                Selector::Positions(_) => {
                    stage2.with_position_matcher(PositionMatcher::from_selector(&sel))
                }
            }
        } else {
            stage2.with_matcher(Box::new(AllMatcher))
        };

        // Expander.
        let stage4 = match self.context {
            Some(n) if n > 0 => stage3.with_expander(Box::new(LineContext::new(n))),
            _ => stage3.with_expander(Box::new(NoContext)),
        };

        // Formatter.
        let stage5 = if let Some(n) = self.char_context {
            stage4.with_formatter(Box::new(FragmentFormatter::new(opts, n)))
        } else {
            stage4.with_formatter(Box::new(PlainFormatter::new(opts)))
        };

        Ok(stage5.with_sink(sink))
    }

    /// Build a ready-to-run `App` for stdin input.
    ///
    /// Returns `PositionalWithStdin` when paired with a positional selector
    /// (line:column), which requires a seekable source.
    pub fn into_app_for_stdin(&self, show_filename: bool) -> crate::Result<App<NonSeek>> {
        if let Some(raw) = self.get_selector()
            && raw.contains(':')
        {
            return Err(crate::SelError::PositionalWithStdin);
        }
        let source = StdinSource::new();
        let filename = if show_filename {
            Some("-".to_string())
        } else {
            None
        };
        let sink = self.make_sink()?;
        let color = self.resolve_color(sink.is_terminal());
        let opts = FormatOpts {
            show_line_numbers: !self.no_line_numbers,
            show_filename,
            filename,
            color,
            // Target marker (`> `) only appears in context-aware output.
            target_marker: matches!(self.context, Some(n) if n > 0),
        };

        let stage2 = Stage1::with_nonseekable_source(Box::new(source));
        let stage3 = if let Some(pat) = &self.regex {
            stage2.with_matcher(Box::new(RegexMatcher::new(pat, self.invert)?))
        } else if let Some(raw) = self.get_selector() {
            let sel = Selector::parse(&raw)?;
            match sel {
                Selector::All => stage2.with_matcher(Box::new(AllMatcher)),
                Selector::LineNumbers(_) => {
                    stage2.with_matcher(Box::new(LineMatcher::from_selector(&sel)))
                }
                Selector::Positions(_) => return Err(crate::SelError::PositionalWithStdin),
            }
        } else {
            stage2.with_matcher(Box::new(AllMatcher))
        };

        let stage4 = match self.context {
            Some(n) if n > 0 => stage3.with_expander(Box::new(LineContext::new(n))),
            _ => stage3.with_expander(Box::new(NoContext)),
        };

        let stage5 = if let Some(n) = self.char_context {
            stage4.with_formatter(Box::new(FragmentFormatter::new(opts, n)))
        } else {
            stage4.with_formatter(Box::new(PlainFormatter::new(opts)))
        };

        Ok(stage5.with_sink(sink))
    }
}

/// Color output mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    /// Always colorize output.
    Always,
    /// Never colorize output.
    Never,
}

impl ColorMode {
    /// Returns true if coloring should be applied.
    pub fn should_colorize(&self) -> bool {
        matches!(self, Self::Always)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_with_selector() {
        let cli = Cli::parse_from(["sel", "10-20", "file.txt"]);
        assert_eq!(cli.get_selector(), Some("10-20".to_string()));
        assert_eq!(cli.get_files().len(), 1);
        assert_eq!(cli.get_files()[0], PathBuf::from("file.txt"));
    }

    #[test]
    fn test_cli_without_selector() {
        let cli = Cli::parse_from(["sel", "file.txt"]);
        assert_eq!(cli.get_selector(), None);
        assert_eq!(cli.get_files().len(), 1);
        assert_eq!(cli.get_files()[0], PathBuf::from("file.txt"));
    }

    #[test]
    fn test_cli_with_context() {
        let cli = Cli::parse_from(["sel", "-c", "3", "42", "file.txt"]);
        assert_eq!(cli.context, Some(3));
        assert_eq!(cli.get_selector(), Some("42".to_string()));
        assert_eq!(cli.get_files().len(), 1);
    }

    #[test]
    fn test_cli_regex_mode() {
        let cli = Cli::parse_from(["sel", "-e", "ERROR", "log.txt"]);
        assert_eq!(cli.regex, Some("ERROR".to_string()));
        assert_eq!(cli.get_selector(), None);
        assert_eq!(cli.get_files().len(), 1);
        assert_eq!(cli.get_files()[0], PathBuf::from("log.txt"));
    }

    #[test]
    fn test_cli_regex_multiple_files() {
        let cli = Cli::parse_from(["sel", "-e", "ERROR", "log1.txt", "log2.txt"]);
        assert_eq!(cli.regex, Some("ERROR".to_string()));
        assert_eq!(cli.get_files().len(), 2);
    }

    #[test]
    fn test_looks_like_selector() {
        let cli = Cli::parse_from(["sel", "file.txt"]);
        assert!(cli.looks_like_selector("42"));
        assert!(cli.looks_like_selector("10-20"));
        assert!(cli.looks_like_selector("1,5,10-15"));
        assert!(cli.looks_like_selector("23:260"));
        assert!(!cli.looks_like_selector("file.txt"));
        assert!(!cli.looks_like_selector(""));
        assert!(!cli.looks_like_selector(":260"));
        assert!(!cli.looks_like_selector("23:"));
    }
}
