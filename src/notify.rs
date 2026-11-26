//! Notification queue, worker, dropped-event log, and zbus backend.
//!
//! The main loop enqueues events. A dedicated worker owns the D-Bus connection
//! and drains the bounded per-source queue.
