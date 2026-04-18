//! Output sinks.

pub mod file;
pub mod stdout;

pub use file::FileSink;
pub use stdout::StdoutSink;

use std::io::{self, Write};

/// A sink is a buffered writer that may know whether it's a terminal.
pub trait Sink: Write {
    /// Is this sink attached to a terminal? Used for `--color=auto`.
    fn is_terminal(&self) -> bool;

    /// Finalize output — flush and surface any error.
    fn finish(self: Box<Self>) -> io::Result<()>;
}
