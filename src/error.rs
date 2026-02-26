//! Error types for `sel`.

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for the `sel` utility.
#[derive(Error, Debug)]
pub enum SelError {
    /// File not found or inaccessible.
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    /// Invalid selector syntax.
    #[error("Invalid selector: {0}")]
    InvalidSelector(String),

    /// Mixed positional and non-positional selectors.
    #[error("Cannot mix positional (L:C) and non-positional selectors")]
    MixedSelectors,

    /// Char context option used without positional selector or -e.
    #[error("Option -n requires positional selector (L:C) or -e flag")]
    CharContextWithoutPosition,

    /// Invalid regular expression.
    #[error("Invalid regex: {0}")]
    InvalidRegex(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error with message.
    #[error("{0}")]
    Message(String),
}

/// Result type alias for `sel`.
pub type Result<T> = std::result::Result<T, SelError>;
