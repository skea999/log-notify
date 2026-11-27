//! `log-notify` library.
//!
//! The daemon tails configured log files, matches one regex per source, and
//! queues freedesktop notifications for the notifier worker.

#![forbid(unsafe_code)]

pub mod config;
pub mod cooldown;
pub mod lock;
pub mod notify;
pub mod pattern;
pub mod tail;
pub mod watch;

/// Crate version from Cargo metadata.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
