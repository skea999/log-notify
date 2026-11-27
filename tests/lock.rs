//! Single-instance lock tests.

mod common;

use common::TestDir;
use log_notify::lock::{Lock, LockError};

#[test]
fn lock_is_exclusive_and_released_on_drop() {
    let dir = TestDir::new("lock");
    let path = dir.file("log-notify.lock");

    let first = Lock::acquire_at(&path).expect("first lock");
    let error = Lock::acquire_at(&path).expect_err("second lock must fail");
    assert!(matches!(error, LockError::AlreadyRunning { .. }));

    drop(first);
    let third = Lock::acquire_at(&path).expect("lock after release");
    assert_eq!(third.path(), path);
}

#[test]
fn missing_runtime_directory_is_reported() {
    let dir = TestDir::new("lock");
    let path = dir.path().join("missing/log-notify.lock");

    let error = Lock::acquire_at(&path).expect_err("missing dir");

    assert!(matches!(error, LockError::MissingDir { .. }));
}
