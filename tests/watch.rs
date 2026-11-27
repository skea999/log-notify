//! Directory watch and rotation tests.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::fs;
use std::path::PathBuf;

use common::{TestDir, append, line};
use log_notify::watch::{TailedLine, Wake, Watcher};

fn new_watcher(paths: &[PathBuf]) -> Watcher {
    Watcher::new(paths, 8192).expect("create watcher")
}

fn ready_watcher(dir: &TestDir) -> (PathBuf, Watcher) {
    let path = dir.file("app.log");
    fs::write(&path, b"old\n").expect("create file");
    let watcher = new_watcher(std::slice::from_ref(&path));
    (path, watcher)
}

fn lines(wake: Wake) -> Vec<TailedLine> {
    match wake {
        Wake::Lines(lines) => lines,
        other => panic!("expected lines, got {other:?}"),
    }
}

fn expect_line(wake: Wake, expected: &str) {
    assert_eq!(
        lines(wake),
        vec![TailedLine {
            source: 0,
            line: line(expected, false),
        }]
    );
}

fn open_late_source(watcher: &mut Watcher, path: &std::path::Path) {
    fs::write(path, b"old\n").expect("create source");
    assert_eq!(watcher.wait().expect("creation event"), Wake::Idle);

    append(path, b"new\n");
    expect_line(watcher.wait().expect("new line"), "new");
}

fn first_line(watcher: &mut Watcher, path: &std::path::Path) {
    append(path, b"one\n");
    expect_line(watcher.wait().expect("first line"), "one");
}

#[test]
fn emits_appended_line() {
    let dir = TestDir::new("watch");
    let (path, mut watcher) = ready_watcher(&dir);

    append(&path, b"hello\n");
    expect_line(watcher.wait().expect("wait for line"), "hello");
}

#[test]
fn missing_source_is_watched_and_opened_at_end() {
    let dir = TestDir::new("watch");
    let path = dir.file("missing.log");
    let mut watcher = new_watcher(std::slice::from_ref(&path));

    let warnings = watcher.take_warnings();
    assert!(warnings.iter().any(|warning| warning.contains("not found")));

    open_late_source(&mut watcher, &path);
}

#[test]
fn intermediate_directories_are_followed() {
    let dir = TestDir::new("watch");
    let path = dir.path().join("nested/deep/app.log");
    let mut watcher = new_watcher(std::slice::from_ref(&path));

    fs::create_dir_all(path.parent().expect("parent")).expect("create dirs");
    open_late_source(&mut watcher, &path);
}

#[test]
fn copytruncate_resets_and_reads_new_content() {
    let dir = TestDir::new("watch");
    let (path, mut watcher) = ready_watcher(&dir);
    first_line(&mut watcher, &path);

    fs::write(&path, b"x\n").expect("truncate");
    expect_line(watcher.wait().expect("truncate event"), "x");
}

#[test]
fn inode_replacement_reopens_at_end() {
    let dir = TestDir::new("watch");
    let (path, mut watcher) = ready_watcher(&dir);
    first_line(&mut watcher, &path);

    let replacement = dir.file("replacement.log");
    fs::write(&replacement, b"new\n").expect("create replacement");
    fs::rename(&replacement, &path).expect("replace file");
    assert_eq!(watcher.wait().expect("replacement event"), Wake::Idle);

    append(&path, b"two\n");
    expect_line(watcher.wait().expect("line after replacement"), "two");
}

#[test]
fn delete_and_recreate_recovers() {
    let dir = TestDir::new("watch");
    let (path, mut watcher) = ready_watcher(&dir);

    fs::remove_file(&path).expect("remove file");
    assert_eq!(watcher.wait().expect("delete event"), Wake::Idle);

    fs::write(&path, b"new\n").expect("recreate file");
    assert_eq!(watcher.wait().expect("create event"), Wake::Idle);

    append(&path, b"next\n");
    expect_line(watcher.wait().expect("line after recreate"), "next");
}

#[test]
fn multiple_sources_keep_their_indices() {
    let dir = TestDir::new("watch");
    let first = dir.file("first.log");
    let second = dir.file("second.log");
    fs::write(&first, b"").expect("create first");
    fs::write(&second, b"").expect("create second");
    let mut watcher = new_watcher(&[first.clone(), second.clone()]);

    append(&first, b"one\n");
    append(&second, b"two\n");
    let found = lines(watcher.wait().expect("both lines"));

    assert!(found.contains(&TailedLine {
        source: 0,
        line: line("one", false),
    }));
    assert!(found.contains(&TailedLine {
        source: 1,
        line: line("two", false),
    }));
}

#[test]
fn shutdown_handle_wakes_the_watcher() {
    let mut watcher = new_watcher(&[]);
    let handle = watcher.shutdown_handle();

    handle.trigger();

    assert_eq!(watcher.wait().expect("shutdown wake"), Wake::Shutdown);
}
