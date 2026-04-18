//! # sel — Select Slices from Text Files
//!
//! `sel` — компактная консольная утилита для быстрого извлечения фрагментов
//! текстовых файлов по номерам строк, диапазонам, позициям или регулярным выражениям.

pub mod cli;
pub mod error;
pub mod matcher;
pub mod output;
pub mod reader;
pub mod selector;
pub mod source;
pub mod types;

pub use error::{Result, SelError};
pub use matcher::{AllMatcher, LineMatcher, Matcher, PositionMatcher, RegexMatcher};
pub use selector::{LineSpec, Position, Selector};
pub use types::{Emit, Line, MatchInfo, Role};
