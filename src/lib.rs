//! # sel — Select Slices from Text Files
//!
//! `sel` — компактная консольная утилита для быстрого извлечения фрагментов
//! текстовых файлов по номерам строк, диапазонам, позициям или регулярным выражениям.

pub mod app;
pub mod cli;
pub mod context;
pub mod error;
pub mod format;
pub mod matcher;
pub mod pipeline;
pub mod selector;
pub mod sink;
pub mod source;
pub mod types;

pub use app::{App, Stage1};
pub use context::{EmitOwned, Expander, LineContext, NoContext};
pub use error::{Result, SelError};
pub use format::{FormatOpts, Formatter, FragmentFormatter, PlainFormatter};
pub use matcher::{AllMatcher, LineMatcher, Matcher, PositionMatcher, RegexMatcher};
pub use pipeline::run;
pub use selector::{LineSpec, Position, Selector};
pub use sink::{Sink, StdoutSink};
pub use types::{Emit, Line, MatchInfo, Role};
