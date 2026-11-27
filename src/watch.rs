//! Directory watches and the file-event loop.
//!
//! Sources are watched through their nearest existing ancestor directory. This
//! covers missing files, copytruncate, inode replacement, and delete/recreate
//! without a separate file watch.

use std::collections::HashMap;
use std::io;
use std::os::fd::AsFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use inotify::{EventMask, Inotify, WatchDescriptor, WatchMask};
use rustix::event::{PollFd, PollFlags, poll};
use rustix::fd::OwnedFd;
use rustix::pipe::{PipeFlags, pipe_with};

use crate::tail::{Line, TailError, Tailer};

/// One line emitted by a watched source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TailedLine {
    /// Index into the path list passed to [`Watcher::new`].
    pub source: usize,
    /// Complete line.
    pub line: Line,
}

/// Result of waiting for file events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wake {
    /// Lines ready for matching.
    Lines(Vec<TailedLine>),
    /// An event was processed but produced no complete line.
    Idle,
    /// Shutdown was requested.
    Shutdown,
}

/// Handle that wakes a blocking [`Watcher::wait`].
#[derive(Debug, Clone)]
pub struct ShutdownHandle {
    writer: Arc<OwnedFd>,
}

impl ShutdownHandle {
    /// Request shutdown. Best effort; repeated calls are harmless.
    pub fn trigger(&self) {
        let _ = rustix::io::write(&*self.writer, &[1]);
    }
}

/// Per-source watch state.
#[derive(Debug)]
struct SourceState {
    path: PathBuf,
    tailer: Option<Tailer>,
}

/// Watches sources and turns file events into complete lines.
#[derive(Debug)]
pub struct Watcher {
    inotify: Inotify,
    sources: Vec<SourceState>,
    dir_watches: HashMap<PathBuf, WatchDescriptor>,
    warnings: Vec<String>,
    shutdown_reader: OwnedFd,
    shutdown_writer: Arc<OwnedFd>,
    line_cap: usize,
}

impl Watcher {
    /// Create a watcher for the given source paths.
    ///
    /// Missing sources are recorded as warnings and their nearest existing
    /// ancestor is watched. Permission and other open errors are fatal.
    pub fn new(paths: &[PathBuf], line_cap: usize) -> Result<Self, TailError> {
        let inotify = Inotify::init().map_err(inotify_error)?;
        let (shutdown_reader, shutdown_writer) =
            pipe_with(PipeFlags::CLOEXEC | PipeFlags::NONBLOCK).map_err(errno_error)?;

        let mut watcher = Self {
            inotify,
            sources: Vec::with_capacity(paths.len()),
            dir_watches: HashMap::new(),
            warnings: Vec::new(),
            shutdown_reader,
            shutdown_writer: Arc::new(shutdown_writer),
            line_cap,
        };

        for path in paths {
            watcher.add_source(path)?;
        }
        for index in 0..watcher.sources.len() {
            watcher.ensure_watch(index);
        }

        Ok(watcher)
    }

    /// Clone a handle that wakes [`Watcher::wait`] and returns [`Wake::Shutdown`].
    #[must_use]
    pub fn shutdown_handle(&self) -> ShutdownHandle {
        ShutdownHandle {
            writer: Arc::clone(&self.shutdown_writer),
        }
    }

    /// Take accumulated warnings for the caller to log.
    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    /// Block until a file event produces lines or shutdown is requested.
    pub fn wait(&mut self) -> Result<Wake, TailError> {
        loop {
            let (inotify_ready, shutdown_ready) = self.poll()?;
            if shutdown_ready {
                self.drain_shutdown()?;
                return Ok(Wake::Shutdown);
            }
            if !inotify_ready {
                continue;
            }

            let mut buffer = [0_u8; 4096];
            let mut overflow = false;
            {
                let events = match self.inotify.read_events(&mut buffer) {
                    Ok(events) => events,
                    Err(source) if source.kind() == io::ErrorKind::WouldBlock => continue,
                    Err(source) => return Err(inotify_error(source)),
                };
                for event in events {
                    if event.mask.contains(EventMask::Q_OVERFLOW) {
                        overflow = true;
                    }
                    if event.mask.intersects(
                        EventMask::DELETE_SELF | EventMask::MOVE_SELF | EventMask::IGNORED,
                    ) {
                        let watch = event.wd;
                        self.dir_watches.retain(|_, current| current != &watch);
                    }
                }
            }
            if overflow {
                self.warnings
                    .push("inotify queue overflow; re-reading from offsets".to_owned());
            }

            let mut lines = Vec::new();
            self.reconcile_all(&mut lines);
            if lines.is_empty() {
                return Ok(Wake::Idle);
            }
            return Ok(Wake::Lines(lines));
        }
    }

    fn add_source(&mut self, path: &Path) -> Result<(), TailError> {
        let tailer = match Tailer::open(path, self.line_cap) {
            Ok(tailer) => Some(tailer),
            Err(TailError::Open { source, .. }) if source.kind() == io::ErrorKind::NotFound => {
                self.warnings.push(format!(
                    "source {} not found; watching nearest ancestor",
                    path.display()
                ));
                None
            }
            Err(error) => return Err(error),
        };
        self.sources.push(SourceState {
            path: path.to_path_buf(),
            tailer,
        });
        Ok(())
    }

    fn poll(&self) -> Result<(bool, bool), TailError> {
        let inotify_fd = self.inotify.as_fd();
        let shutdown_fd = self.shutdown_reader.as_fd();
        let mut fds = [
            PollFd::new(&inotify_fd, PollFlags::IN),
            PollFd::new(&shutdown_fd, PollFlags::IN),
        ];
        let ready = poll(&mut fds, None).map_err(errno_error)?;
        if ready == 0 {
            return Ok((false, false));
        }
        let [inotify_poll, shutdown_poll] = fds;
        Ok((
            inotify_poll.revents().contains(PollFlags::IN),
            shutdown_poll.revents().contains(PollFlags::IN),
        ))
    }

    fn drain_shutdown(&self) -> Result<(), TailError> {
        let mut buffer = [0_u8; 16];
        loop {
            match rustix::io::read(&self.shutdown_reader, &mut buffer) {
                Ok(0) | Err(rustix::io::Errno::AGAIN) => return Ok(()),
                Ok(_) => {}
                Err(error) => return Err(errno_error(error)),
            }
        }
    }

    fn ensure_watch(&mut self, index: usize) {
        let Some(source) = self.sources.get(index) else {
            return;
        };
        let path = source.path.clone();
        let Some(directory) = nearest_existing_dir(&path) else {
            self.warnings
                .push(format!("no existing ancestor for {}", path.display()));
            return;
        };
        if self.dir_watches.contains_key(&directory) {
            return;
        }
        match self.inotify.watches().add(&directory, directory_mask()) {
            Ok(watch) => {
                self.dir_watches.insert(directory, watch);
            }
            Err(source) => {
                self.warnings
                    .push(format!("cannot watch {}: {source}", directory.display()));
            }
        }
    }

    fn set_tailer(&mut self, index: usize, tailer: Tailer) {
        if let Some(source) = self.sources.get_mut(index) {
            source.tailer = Some(tailer);
        }
    }

    fn reconcile_all(&mut self, lines: &mut Vec<TailedLine>) {
        for index in 0..self.sources.len() {
            self.reconcile_one(index, lines);
        }
    }

    fn reconcile_one(&mut self, index: usize, lines: &mut Vec<TailedLine>) {
        let Some(source) = self.sources.get(index) else {
            return;
        };
        let path = source.path.clone();
        let tailer = self
            .sources
            .get_mut(index)
            .and_then(|source| source.tailer.take());
        let mut warnings = Vec::new();
        let mut needs_watch = false;

        if let Some(mut tailer) = tailer {
            match std::fs::metadata(&path) {
                Ok(metadata) => {
                    if metadata.dev() != tailer.dev() || metadata.ino() != tailer.ino() {
                        warnings.push(format!(
                            "source {} replaced; reopening at end",
                            path.display()
                        ));
                        match Tailer::open(&path, self.line_cap) {
                            Ok(new_tailer) => self.set_tailer(index, new_tailer),
                            Err(error) => warnings.push(format!("{error}")),
                        }
                    } else {
                        if metadata.len() < tailer.offset() {
                            warnings.push(format!(
                                "source {} shrank; resetting offset",
                                path.display()
                            ));
                            if let Err(error) = tailer.reset_to_start() {
                                warnings.push(format!("{error}"));
                            }
                        }
                        match tailer.read_new_lines() {
                            Ok(found) => {
                                lines.extend(found.into_iter().map(|line| TailedLine {
                                    source: index,
                                    line,
                                }));
                                self.set_tailer(index, tailer);
                            }
                            Err(error) => {
                                warnings.push(format!("{error}"));
                                self.set_tailer(index, tailer);
                            }
                        }
                    }
                }
                Err(source) if source.kind() == io::ErrorKind::NotFound => {
                    warnings.push(format!(
                        "source {} disappeared; waiting for recreation",
                        path.display()
                    ));
                    needs_watch = true;
                }
                Err(source) => {
                    warnings.push(format!("cannot stat {}: {source}", path.display()));
                    self.set_tailer(index, tailer);
                }
            }
        } else if path.is_file() {
            match Tailer::open(&path, self.line_cap) {
                Ok(tailer) => {
                    warnings.push(format!(
                        "source {} appeared; opening at end",
                        path.display()
                    ));
                    self.set_tailer(index, tailer);
                    needs_watch = true;
                }
                Err(error) => warnings.push(format!("{error}")),
            }
        } else {
            needs_watch = true;
        }

        self.warnings.extend(warnings);
        if needs_watch {
            self.ensure_watch(index);
        }
    }
}

fn nearest_existing_dir(path: &Path) -> Option<PathBuf> {
    let mut current = path.parent();
    while let Some(directory) = current {
        if directory.is_dir() {
            return Some(directory.to_path_buf());
        }
        current = directory.parent();
    }
    None
}

fn directory_mask() -> WatchMask {
    WatchMask::MODIFY
        | WatchMask::CLOSE_WRITE
        | WatchMask::CREATE
        | WatchMask::DELETE
        | WatchMask::MOVED_TO
        | WatchMask::MOVED_FROM
        | WatchMask::DELETE_SELF
        | WatchMask::MOVE_SELF
}

fn inotify_error(source: io::Error) -> TailError {
    TailError::Inotify { source }
}

fn errno_error(error: rustix::io::Errno) -> TailError {
    TailError::Inotify {
        source: io::Error::from_raw_os_error(error.raw_os_error()),
    }
}
