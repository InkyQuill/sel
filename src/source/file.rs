//! File-backed `Source`.

use super::Source;
use crate::error::SelError;
use crate::{Line, Result};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct FileSource {
    reader: BufReader<File>,
    label: String,
    path: PathBuf,
    line_no: u64,
}

impl FileSource {
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path).map_err(|source| SelError::Io {
            path: path.display().to_string(),
            source,
        })?;
        Ok(Self {
            reader: BufReader::new(file),
            label: path.display().to_string(),
            path: path.to_path_buf(),
            line_no: 0,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Source for FileSource {
    fn next_line(&mut self) -> Result<Option<Line>> {
        let mut buf: Vec<u8> = Vec::new();
        let n = self
            .reader
            .read_until(b'\n', &mut buf)
            .map_err(|source| SelError::Io {
                path: self.label.clone(),
                source,
            })?;
        if n == 0 {
            return Ok(None);
        }
        // Strip trailing \n and optional \r
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
        &self.label
    }

    fn is_seekable(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn reads_three_lines_numbered() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "alpha").unwrap();
        writeln!(f, "beta").unwrap();
        writeln!(f, "gamma").unwrap();

        let mut src = FileSource::open(f.path()).unwrap();
        let l1 = src.next_line().unwrap().unwrap();
        let l2 = src.next_line().unwrap().unwrap();
        let l3 = src.next_line().unwrap().unwrap();
        assert!(src.next_line().unwrap().is_none());

        assert_eq!(l1.no, 1);
        assert_eq!(&l1.bytes, b"alpha");
        assert_eq!(l2.no, 2);
        assert_eq!(&l2.bytes, b"beta");
        assert_eq!(l3.no, 3);
        assert_eq!(&l3.bytes, b"gamma");
    }

    #[test]
    fn handles_crlf() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"one\r\ntwo\r\n").unwrap();

        let mut src = FileSource::open(f.path()).unwrap();
        let l1 = src.next_line().unwrap().unwrap();
        assert_eq!(&l1.bytes, b"one");
    }

    #[test]
    fn nonexistent_file_returns_io_error_with_path() {
        let err = FileSource::open(Path::new("/nonexistent-xyz-123")).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("nonexistent-xyz-123"));
    }
}
