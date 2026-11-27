//! Shared test fixtures.

#![allow(clippy::expect_used, clippy::redundant_pub_crate, dead_code)]

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use log_notify::tail::Line;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Isolated directory under `artifacts/tests/<kind>`.
pub(crate) struct TestDir {
    path: PathBuf,
}

impl TestDir {
    pub(crate) fn new(kind: &str) -> Self {
        let base = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("artifacts/tests")
            .join(kind);
        fs::create_dir_all(&base).expect("create test base");
        let id = format!(
            "{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let path = base.join(id);
        fs::create_dir_all(&path).expect("create test dir");
        Self { path }
    }

    pub(crate) fn file(&self, name: &str) -> PathBuf {
        self.path.join(name)
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Append bytes to a file, creating it if needed.
pub(crate) fn append(path: &Path, data: &[u8]) {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .expect("open for append");
    file.write_all(data).expect("append data");
}

/// Build a complete line expected by tail tests.
pub(crate) fn line(text: &str, truncated: bool) -> Line {
    Line {
        text: text.to_owned(),
        truncated,
    }
}
