//! CLI smoke tests.

use std::process::Command;

#[test]
fn version_prints_package_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_log-notify"))
        .arg("--version")
        .output()
        .expect("run log-notify --version");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        concat!("log-notify ", env!("CARGO_PKG_VERSION"))
    );
}
