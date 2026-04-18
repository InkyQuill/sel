//! Buffered stdout sink.

use super::Sink;
use std::io::{self, BufWriter, IsTerminal, Stdout, StdoutLock, Write};

pub struct StdoutSink {
    writer: BufWriter<StdoutLock<'static>>,
    is_tty: bool,
}

impl StdoutSink {
    pub fn new() -> Self {
        let out: Stdout = io::stdout();
        let is_tty = out.is_terminal();
        // SAFETY: we intentionally leak the lock for the lifetime of the process.
        // The sink is consumed once at the end of `main`, which is the same lifetime.
        let lock: StdoutLock<'static> = Box::leak(Box::new(out)).lock();
        Self {
            writer: BufWriter::with_capacity(64 * 1024, lock),
            is_tty,
        }
    }
}

impl Default for StdoutSink {
    fn default() -> Self {
        Self::new()
    }
}

impl Write for StdoutSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl Sink for StdoutSink {
    fn is_terminal(&self) -> bool {
        self.is_tty
    }
    fn finish(self: Box<Self>) -> io::Result<()> {
        let mut this = *self;
        this.writer.flush()
    }
}
