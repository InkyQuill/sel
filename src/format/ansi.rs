//! Minimal ANSI escape helpers — replaces the termcolor crate.

pub const GREEN: &str = "\x1b[32m";
pub const INVERSE: &str = "\x1b[7m";
pub const RESET: &str = "\x1b[0m";

/// Wrap `text` in `code` … RESET if `enabled`, otherwise pass through.
pub fn paint(enabled: bool, code: &str, text: &str) -> String {
    if enabled {
        format!("{code}{text}{RESET}")
    } else {
        text.to_string()
    }
}
