//! CLI flags and lifecycle tests.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::fs;
use std::io::Read;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use common::{TestDir, append};

fn wait_with_timeout(mut child: Child, timeout: Duration) -> (ExitStatus, String, String) {
    let mut stdout = child.stdout.take().expect("child stdout");
    let mut stderr = child.stderr.take().expect("child stderr");
    let deadline = Instant::now() + timeout;

    loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            let mut out = String::new();
            stdout.read_to_string(&mut out).expect("read stdout");
            let mut err = String::new();
            stderr.read_to_string(&mut err).expect("read stderr");
            return (status, out, err);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("child did not exit within {timeout:?}");
        }
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn rejects_unknown_flag() {
    let output = Command::new(env!("CARGO_BIN_EXE_log-notify"))
        .arg("--bogus")
        .output()
        .expect("run binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("unexpected argument"), "{stderr}");
}

#[test]
fn reports_missing_explicit_config() {
    let dir = TestDir::new("cli");
    let path = dir.file("missing.toml");
    let output = Command::new(env!("CARGO_BIN_EXE_log-notify"))
        .arg("--config")
        .arg(&path)
        .output()
        .expect("run binary");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing.toml"), "{stderr}");
}

#[test]
fn dry_run_once_prints_first_match() {
    let dir = TestDir::new("cli");
    let log = dir.file("app.log");
    fs::write(&log, b"").expect("create log");
    let config = dir.file("log-notify.toml");
    let dropped = dir.file("dropped.log");
    let text = format!(
        r#"
[general]
line_cap_bytes = 8192
dropped_log = "{}"

[[source]]
path = "{}"
pattern = "FOUND$"
summary = "found"
body = "{{line}}"
urgency = "critical"
"#,
        dropped.display(),
        log.display()
    );
    fs::write(&config, text).expect("write config");

    let child = Command::new(env!("CARGO_BIN_EXE_log-notify"))
        .arg("--config")
        .arg(&config)
        .arg("--dry-run")
        .arg("--once")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn binary");

    thread::sleep(Duration::from_millis(750));
    append(&log, b"hello\nFOUND\n");

    let (status, stdout, stderr) = wait_with_timeout(child, Duration::from_secs(10));
    assert!(status.success(), "{stderr}");
    assert!(stdout.contains("found"), "{stdout}");
    assert!(stdout.contains("FOUND"), "{stdout}");
}
