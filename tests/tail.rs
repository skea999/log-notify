//! Tail state machine tests.
#![allow(clippy::expect_used)]

mod common;

use std::fs;

use common::{TestDir, append, line};
use log_notify::tail::{TailError, Tailer};

fn open_empty(dir: &TestDir, line_cap: usize) -> (std::path::PathBuf, Tailer) {
    let path = dir.file("app.log");
    fs::write(&path, b"").expect("create file");
    let tailer = Tailer::open(&path, line_cap).expect("open tailer");
    (path, tailer)
}

#[test]
fn starts_at_end_and_reads_appended_lines() {
    let dir = TestDir::new("tail");
    let path = dir.file("app.log");
    fs::write(&path, b"old\n").expect("write old");

    let mut tailer = Tailer::open(&path, 8192).expect("open tailer");
    assert_eq!(tailer.offset(), 4);
    assert!(tailer.read_new_lines().expect("initial read").is_empty());

    append(&path, b"new\n");
    let lines = tailer.read_new_lines().expect("read append");

    assert_eq!(lines, vec![line("new", false)]);
    assert_eq!(tailer.offset(), 8);
}

#[test]
fn buffers_partial_line_until_newline() {
    let dir = TestDir::new("tail");
    let (path, mut tailer) = open_empty(&dir, 8192);

    append(&path, b"par");
    assert!(tailer.read_new_lines().expect("partial read").is_empty());

    append(&path, b"tial\n");
    let lines = tailer.read_new_lines().expect("complete read");

    assert_eq!(lines, vec![line("partial", false)]);
}

#[test]
fn returns_multiple_complete_lines_and_keeps_the_tail() {
    let dir = TestDir::new("tail");
    let (path, mut tailer) = open_empty(&dir, 8192);

    append(&path, b"a\nb\nc");
    let lines = tailer.read_new_lines().expect("read lines");
    assert_eq!(
        lines
            .iter()
            .map(|line| line.text.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b"]
    );

    append(&path, b"\n");
    let lines = tailer.read_new_lines().expect("finish line");
    assert_eq!(lines, vec![line("c", false)]);
}

#[test]
fn caps_long_line_and_marks_it_truncated() {
    let dir = TestDir::new("tail");
    let (path, mut tailer) = open_empty(&dir, 8);

    append(&path, b"123456789012\nnext\n");
    let lines = tailer.read_new_lines().expect("read long line");

    assert_eq!(lines, vec![line("12345678", true), line("next", false),]);
}

#[test]
fn exact_cap_is_not_truncated() {
    let dir = TestDir::new("tail");
    let (path, mut tailer) = open_empty(&dir, 8);

    append(&path, b"12345678\n");
    let lines = tailer.read_new_lines().expect("read exact cap");

    assert_eq!(lines, vec![line("12345678", false)]);
}

#[test]
fn invalid_utf8_is_lossy() {
    let dir = TestDir::new("tail");
    let (path, mut tailer) = open_empty(&dir, 8192);

    append(&path, &[0xff, b'\n']);
    let lines = tailer.read_new_lines().expect("read invalid utf8");

    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].text, "\u{fffd}");
    assert!(!lines[0].truncated);
}

#[test]
fn empty_lines_are_emitted() {
    let dir = TestDir::new("tail");
    let (path, mut tailer) = open_empty(&dir, 8192);

    append(&path, b"\n\n");
    let lines = tailer.read_new_lines().expect("read empty lines");

    assert_eq!(lines, vec![line("", false), line("", false),]);
}

#[test]
fn partial_line_survives_end_of_file() {
    let dir = TestDir::new("tail");
    let (path, mut tailer) = open_empty(&dir, 8192);

    append(&path, b"abc");
    assert!(tailer.read_new_lines().expect("partial read").is_empty());
    assert_eq!(tailer.offset(), 3);

    append(&path, b"def\n");
    let lines = tailer.read_new_lines().expect("finish partial");

    assert_eq!(lines, vec![line("abcdef", false)]);
    assert_eq!(tailer.offset(), 7);
}

#[test]
fn rejects_zero_line_cap() {
    let dir = TestDir::new("tail");
    let path = dir.file("app.log");
    fs::write(&path, b"").expect("create file");

    let err = Tailer::open(&path, 0).expect_err("zero cap");

    assert!(matches!(err, TailError::InvalidLineCap));
}

#[test]
fn reports_open_error_with_path() {
    let dir = TestDir::new("tail");
    let path = dir.file("missing.log");

    let err = Tailer::open(&path, 8192).expect_err("missing file");

    assert!(matches!(err, TailError::Open { .. }));
    assert!(format!("{err}").contains("missing.log"));
}
