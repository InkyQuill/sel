//! Error types for `sel`.

use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Main error type for the `sel` utility.
#[derive(Error, Debug)]
pub enum SelError {
    /// Invalid selector syntax.
    #[error("invalid selector: {0}")]
    InvalidSelector(String),

    /// Invalid regular expression.
    #[error("invalid regex: {0}")]
    InvalidRegex(String),

    /// Positional selectors used with stdin (unseekable).
    #[error("positional selectors require a seekable file; stdin is line-only")]
    PositionalWithStdin,

    /// `--invert-match` used without `--regex`.
    #[error("--invert-match requires --regex")]
    InvertWithoutRegex,

    /// `--char-context` used without a target.
    #[error("--char-context requires --regex or a positional selector")]
    CharContextWithoutTarget,

    /// I/O error with the offending path.
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: io::Error,
    },

    /// Output file already exists and `--force` was not given.
    #[error("output file already exists: {} (use --force to overwrite)", .0.display())]
    OutputExists(PathBuf),
}

/// Result type alias for `sel`.
pub type Result<T> = std::result::Result<T, SelError>;

/// Convert bare `io::Error` into `SelError::Io` with an unknown path.
///
/// This impl is a transitional shim for legacy code paths that propagate
/// raw `io::Error` (output writes, reader internals). Call sites that know
/// their path should construct `SelError::Io { path, source }` directly.
impl From<io::Error> for SelError {
    fn from(source: io::Error) -> Self {
        SelError::Io {
            path: "<unknown>".to_string(),
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::path::PathBuf;

    #[test]
    fn io_error_includes_path() {
        let err = SelError::Io {
            path: "nope.txt".into(),
            source: io::Error::new(io::ErrorKind::NotFound, "no such"),
        };
        let msg = format!("{err}");
        assert!(msg.contains("nope.txt"), "got: {msg}");
    }

    #[test]
    fn positional_with_stdin_has_clear_message() {
        let err = SelError::PositionalWithStdin;
        let msg = format!("{err}");
        assert!(msg.contains("stdin"));
    }

    #[test]
    fn output_exists_names_path() {
        let err = SelError::OutputExists(PathBuf::from("out.txt"));
        let msg = format!("{err}");
        assert!(msg.contains("out.txt"));
        assert!(msg.contains("--force"));
    }
}
