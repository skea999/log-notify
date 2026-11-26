//! `log-notify` CLI entry point.

use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args().any(|arg| arg == "--version" || arg == "-V") {
        println!("log-notify {}", log_notify::VERSION);
        return ExitCode::SUCCESS;
    }

    eprintln!("log-notify: v0.1 implementation in progress");
    ExitCode::FAILURE
}
