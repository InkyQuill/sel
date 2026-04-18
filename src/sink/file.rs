//! File sink with create-new/force behaviour.

use super::Sink;
use crate::Result;
use crate::error::SelError;
use std::fs::{File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

pub struct FileSink {
    writer: BufWriter<File>,
    #[allow(dead_code)]
    path: PathBuf,
}

impl FileSink {
    pub fn create(path: &Path, force: bool) -> Result<Self> {
        let mut opts = OpenOptions::new();
        opts.write(true);
        if force {
            opts.create(true).truncate(true);
        } else {
            opts.create_new(true);
        }
        let file = opts.open(path).map_err(|source| {
            if source.kind() == io::ErrorKind::AlreadyExists {
                SelError::OutputExists(path.to_path_buf())
            } else {
                SelError::Io {
                    path: path.display().to_string(),
                    source,
                }
            }
        })?;
        Ok(Self {
            writer: BufWriter::with_capacity(64 * 1024, file),
            path: path.to_path_buf(),
        })
    }
}

impl Write for FileSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.writer.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl Sink for FileSink {
    fn is_terminal(&self) -> bool {
        false
    }
    fn finish(self: Box<Self>) -> io::Result<()> {
        let mut this = *self;
        this.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn create_new_succeeds_on_fresh_path() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("out.txt");
        let mut s = FileSink::create(&p, false).unwrap();
        s.write_all(b"hi\n").unwrap();
        Box::new(s).finish().unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "hi\n");
    }

    #[test]
    fn create_fails_when_exists_without_force() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("exists.txt");
        std::fs::write(&p, "prior").unwrap();
        let result = FileSink::create(&p, false);
        match result {
            Err(SelError::OutputExists(_)) => {}
            Ok(_) => panic!("expected OutputExists error, got Ok"),
            Err(e) => panic!("expected OutputExists error, got: {e:?}"),
        }
    }

    #[test]
    fn force_truncates_existing() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("force.txt");
        std::fs::write(&p, "old content that is longer than new").unwrap();
        let mut s = FileSink::create(&p, true).unwrap();
        s.write_all(b"new\n").unwrap();
        Box::new(s).finish().unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "new\n");
    }
}
