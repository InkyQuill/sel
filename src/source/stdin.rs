//! Stdin-backed Source.

use super::Source;
use crate::error::SelError;
use crate::{Line, Result};
use std::io::{self, BufRead, BufReader, Stdin, StdinLock};

pub struct StdinSource {
    reader: BufReader<StdinLock<'static>>,
    line_no: u64,
}

impl StdinSource {
    pub fn new() -> Self {
        let stdin: Stdin = io::stdin();
        // SAFETY: mirrors StdoutSink — lock lifetime = process lifetime.
        let lock: StdinLock<'static> = Box::leak(Box::new(stdin)).lock();
        Self {
            reader: BufReader::new(lock),
            line_no: 0,
        }
    }
}

impl Default for StdinSource {
    fn default() -> Self {
        Self::new()
    }
}

impl Source for StdinSource {
    fn next_line(&mut self) -> Result<Option<Line>> {
        let mut buf: Vec<u8> = Vec::new();
        let n = self
            .reader
            .read_until(b'\n', &mut buf)
            .map_err(|source| SelError::Io {
                path: "-".to_string(),
                source,
            })?;
        if n == 0 {
            return Ok(None);
        }
        if buf.ends_with(b"\n") {
            buf.pop();
            if buf.ends_with(b"\r") {
                buf.pop();
            }
        }
        self.line_no += 1;
        Ok(Some(Line::new(self.line_no, buf)))
    }

    fn label(&self) -> &str {
        "-"
    }

    fn is_seekable(&self) -> bool {
        false
    }
}
