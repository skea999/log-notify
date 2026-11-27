//! inotify tailer core.
//!
//! Tracks offsets, partial lines, the line cap, and file identity. Every source
//! starts at end of file. File events and rotation attach in later tasks.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use thiserror::Error;

const READ_BUFFER_BYTES: usize = 8192;

/// One complete log line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Line text, with invalid UTF-8 replaced lossily.
    pub text: String,
    /// True when the line exceeded the cap and its tail was discarded.
    pub truncated: bool,
}

/// Tail open or read failure.
#[derive(Debug, Error)]
pub enum TailError {
    /// The source could not be opened.
    #[error("cannot open {path}: {source}")]
    Open {
        /// Source path.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The source could not be read.
    #[error("cannot read {path}: {source}")]
    Read {
        /// Source path.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The configured line cap is invalid.
    #[error("line cap must be greater than 0")]
    InvalidLineCap,
    /// The inotify instance failed.
    #[error("inotify error: {source}")]
    Inotify {
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// A directory watch could not be added.
    #[error("cannot watch {path}: {source}")]
    Watch {
        /// Watched directory.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

/// Per-source tail state.
#[derive(Debug)]
pub struct Tailer {
    path: PathBuf,
    file: File,
    dev: u64,
    ino: u64,
    offset: u64,
    line_cap: usize,
    partial: Vec<u8>,
    discarding: bool,
}

impl Tailer {
    /// Open a source at end of file.
    pub fn open(path: &Path, line_cap: usize) -> Result<Self, TailError> {
        if line_cap == 0 {
            return Err(TailError::InvalidLineCap);
        }

        let path_buf = path.to_path_buf();
        let mut file = File::open(path).map_err(|source| TailError::Open {
            path: path_buf.clone(),
            source,
        })?;
        let metadata = file.metadata().map_err(|source| TailError::Open {
            path: path_buf.clone(),
            source,
        })?;
        let offset = file
            .seek(SeekFrom::End(0))
            .map_err(|source| TailError::Open {
                path: path_buf.clone(),
                source,
            })?;

        Ok(Self {
            path: path_buf,
            file,
            dev: metadata.dev(),
            ino: metadata.ino(),
            offset,
            line_cap,
            partial: Vec::new(),
            discarding: false,
        })
    }

    /// Source path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Current byte offset (next unread byte).
    #[must_use]
    pub fn offset(&self) -> u64 {
        self.offset
    }

    /// Device id of the open file.
    #[must_use]
    pub fn dev(&self) -> u64 {
        self.dev
    }

    /// Inode of the open file.
    #[must_use]
    pub fn ino(&self) -> u64 {
        self.ino
    }

    /// Read all bytes currently available and return complete lines.
    ///
    /// A trailing partial line is buffered until its newline arrives. Lines
    /// longer than the cap keep the capped prefix and discard the rest until
    /// the newline; the resulting line is marked truncated.
    pub fn read_new_lines(&mut self) -> Result<Vec<Line>, TailError> {
        let mut buffer = [0_u8; READ_BUFFER_BYTES];
        let mut lines = Vec::new();

        loop {
            let read = self
                .file
                .read(&mut buffer)
                .map_err(|source| TailError::Read {
                    path: self.path.clone(),
                    source,
                })?;
            if read == 0 {
                break;
            }
            self.offset += read as u64;
            for &byte in buffer.iter().take(read) {
                self.feed(byte, &mut lines);
            }
        }

        Ok(lines)
    }

    /// Reset to the start of the file after a shrink (copytruncate).
    pub fn reset_to_start(&mut self) -> Result<(), TailError> {
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|source| TailError::Read {
                path: self.path.clone(),
                source,
            })?;
        self.offset = 0;
        self.partial.clear();
        self.discarding = false;
        Ok(())
    }

    fn feed(&mut self, byte: u8, lines: &mut Vec<Line>) {
        if byte == b'\n' {
            let partial = std::mem::take(&mut self.partial);
            lines.push(Line {
                text: String::from_utf8_lossy(&partial).into_owned(),
                truncated: self.discarding,
            });
            self.discarding = false;
            return;
        }

        if self.discarding {
            return;
        }

        if self.partial.len() < self.line_cap {
            self.partial.push(byte);
        } else {
            self.discarding = true;
        }
    }
}
