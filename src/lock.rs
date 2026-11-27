//! Single-instance advisory lock.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use rustix::fs::{FlockOperation, flock};
use thiserror::Error;

/// Held advisory lock. Dropping it releases the lock.
#[derive(Debug)]
pub struct Lock {
    _file: File,
    path: PathBuf,
}

/// Lock acquisition failure.
#[derive(Debug, Error)]
pub enum LockError {
    /// Another instance holds the lock.
    #[error("log-notify is already running (lock {path})")]
    AlreadyRunning {
        /// Lock path.
        path: PathBuf,
    },
    /// The lock file could not be opened or locked.
    #[error("cannot acquire lock {path}: {source}")]
    Io {
        /// Lock path.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },
    /// The runtime directory does not exist.
    #[error("runtime directory missing: {path}")]
    MissingDir {
        /// Missing directory.
        path: PathBuf,
    },
    /// No XDG runtime directory and no `/run/user/<uid>`.
    #[error("cannot determine a runtime directory; set XDG_RUNTIME_DIR")]
    NoRuntimeDir,
}

impl Lock {
    /// Acquire the default user-runtime lock.
    pub fn acquire_default() -> Result<Self, LockError> {
        let path = default_lock_path()?;
        Self::acquire_at(&path)
    }

    /// Acquire a lock at an explicit path.
    pub fn acquire_at(path: &Path) -> Result<Self, LockError> {
        let Some(parent) = path.parent() else {
            return Err(LockError::MissingDir {
                path: path.to_path_buf(),
            });
        };
        if !parent.is_dir() {
            return Err(LockError::MissingDir {
                path: parent.to_path_buf(),
            });
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|source| LockError::Io {
                path: path.to_path_buf(),
                source,
            })?;

        match flock(&file, FlockOperation::NonBlockingLockExclusive) {
            Ok(()) => Ok(Self {
                _file: file,
                path: path.to_path_buf(),
            }),
            Err(rustix::io::Errno::WOULDBLOCK) => Err(LockError::AlreadyRunning {
                path: path.to_path_buf(),
            }),
            Err(source) => Err(LockError::Io {
                path: path.to_path_buf(),
                source: io::Error::from_raw_os_error(source.raw_os_error()),
            }),
        }
    }

    /// Path of the held lock.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Resolve `$XDG_RUNTIME_DIR/log-notify.lock`, then `/run/user/<uid>`.
pub fn default_lock_path() -> Result<PathBuf, LockError> {
    if let Some(directory) = std::env::var_os("XDG_RUNTIME_DIR")
        && !directory.is_empty()
    {
        return Ok(PathBuf::from(directory).join("log-notify.lock"));
    }

    let directory = PathBuf::from(format!("/run/user/{}", rustix::process::getuid().as_raw()));
    if directory.is_dir() {
        Ok(directory.join("log-notify.lock"))
    } else {
        Err(LockError::NoRuntimeDir)
    }
}
