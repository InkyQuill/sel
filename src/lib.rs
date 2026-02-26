//! # sel — Select Slices from Text Files
//!
//! `sel` — компактная консольная утилита для быстрого извлечения фрагментов
//! текстовых файлов по номерам строк, диапазонам, позициям или регулярным выражениям.

pub mod cli;
pub mod error;
pub mod output;
pub mod reader;
pub mod selector;

pub use error::{Result, SelError};
pub use selector::{LineSpec, Position, Selector};
