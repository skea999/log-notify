//! Configuration loading and validation.
//!
//! TOML sources, regex compilation, queue capacities, and the dropped-event log
//! are validated at startup. Errors carry the config file, line, and reason.
