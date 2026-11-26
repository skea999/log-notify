//! Config parsing and validation tests.

use std::path::{Path, PathBuf};

use log_notify::config::{
    Config, ConfigError, DEFAULT_DROPPED_LOG, DEFAULT_LINE_CAP_BYTES, DEFAULT_QUEUE_CAPACITY,
    MAX_QUEUE_CAPACITY, SYSTEM_CONFIG_PATH, Urgency, default_paths, load, load_from,
};

const FULL_CONFIG: &str = r#"
[general]
line_cap_bytes = 4096
dropped_log = "/var/log/custom-dropped.log"

[[source]]
path = "/var/log/fail2ban.log"
pattern = ' Ban [0-9]+\.[0-9]+\.[0-9]+\.[0-9]+ '
summary = "fail2ban ban"
body = "{line}"
urgency = "critical"
cooldown_secs = 60
queue_capacity = 20
enabled = false

[[source]]
path = "/var/log/clamav/clamd.log"
pattern = ' FOUND$'
summary = "clamav detection"
"#;

#[test]
fn parses_all_fields_and_preserves_source_order() {
    let config = Config::parse(Path::new("test.toml"), FULL_CONFIG).expect("valid config");

    assert_eq!(config.general.line_cap_bytes, 4096);
    assert_eq!(
        config.general.dropped_log,
        PathBuf::from("/var/log/custom-dropped.log")
    );
    assert_eq!(config.sources.len(), 2);
    assert_eq!(
        config.sources[0].path,
        PathBuf::from("/var/log/fail2ban.log")
    );
    assert_eq!(config.sources[0].urgency, Urgency::Critical);
    assert_eq!(config.sources[0].cooldown_secs, 60);
    assert_eq!(config.sources[0].queue_capacity, 20);
    assert!(!config.sources[0].enabled);
    assert_eq!(
        config.sources[1].path,
        PathBuf::from("/var/log/clamav/clamd.log")
    );
}

#[test]
fn applies_documented_defaults() {
    let text = r#"
[[source]]
path = "/var/log/a.log"
pattern = "x"
summary = "s"
"#;
    let config = Config::parse(Path::new("test.toml"), text).expect("valid config");

    assert_eq!(config.general.line_cap_bytes, DEFAULT_LINE_CAP_BYTES);
    assert_eq!(
        config.general.dropped_log,
        PathBuf::from(DEFAULT_DROPPED_LOG)
    );
    let source = &config.sources[0];
    assert_eq!(source.body, "");
    assert_eq!(source.urgency, Urgency::Normal);
    assert_eq!(source.cooldown_secs, 0);
    assert_eq!(source.queue_capacity, DEFAULT_QUEUE_CAPACITY);
    assert!(source.enabled);
}

#[test]
fn rejects_unknown_fields() {
    let text = r#"
[general]
bogus = 1

[[source]]
path = "/var/log/a.log"
pattern = "x"
summary = "s"
"#;
    let err = Config::parse(Path::new("test.toml"), text).expect_err("unknown field");

    assert!(matches!(err, ConfigError::Parse { .. }));
    assert!(format!("{err}").contains("unknown field"));
}

#[test]
fn rejects_missing_required_field() {
    let text = r#"
[[source]]
path = "/var/log/a.log"
pattern = "x"
"#;
    let err = Config::parse(Path::new("test.toml"), text).expect_err("missing summary");

    assert!(matches!(err, ConfigError::Parse { .. }));
    assert!(format!("{err}").contains("missing field"));
}

#[test]
fn rejects_bad_urgency() {
    let text = r#"
[[source]]
path = "/var/log/a.log"
pattern = "x"
summary = "s"
urgency = "urgent"
"#;
    let err = Config::parse(Path::new("test.toml"), text).expect_err("bad urgency");

    assert!(matches!(err, ConfigError::Parse { .. }));
    assert!(format!("{err}").contains("unknown variant"));
}

#[test]
fn rejects_empty_source_list() {
    let text = r"
[general]
line_cap_bytes = 1024
";
    let err = Config::parse(Path::new("test.toml"), text).expect_err("no sources");

    assert!(matches!(err, ConfigError::Validation { line: 1, .. }));
    assert!(format!("{err}").contains("at least one [[source]]"));
}

#[test]
fn rejects_out_of_range_queue_capacity() {
    let text = r#"
[[source]]
path = "/var/log/a.log"
pattern = "x"
summary = "s"
queue_capacity = 101
"#;
    let err = Config::parse(Path::new("test.toml"), text).expect_err("queue too large");

    assert!(matches!(err, ConfigError::Validation { .. }));
    let message = format!("{err}");
    assert!(message.contains(":6:"), "{message}");
    assert!(
        message.contains(&MAX_QUEUE_CAPACITY.to_string()),
        "{message}"
    );
}

#[test]
fn rejects_empty_required_text() {
    let text = r#"
[[source]]
path = ""
pattern = "x"
summary = "s"
"#;
    let err = Config::parse(Path::new("test.toml"), text).expect_err("empty path");

    assert!(matches!(err, ConfigError::Validation { .. }));
    assert!(format!("{err}").contains("path must not be empty"));
}

#[test]
fn rejects_empty_dropped_log() {
    let text = r#"
[general]
dropped_log = ""

[[source]]
path = "/var/log/a.log"
pattern = "x"
summary = "s"
"#;
    let err = Config::parse(Path::new("test.toml"), text).expect_err("empty dropped log");

    assert!(matches!(err, ConfigError::Validation { .. }));
    assert!(format!("{err}").contains("dropped_log must not be empty"));
}

#[test]
fn rejects_zero_line_cap() {
    let text = r#"
[general]
line_cap_bytes = 0

[[source]]
path = "/var/log/a.log"
pattern = "x"
summary = "s"
"#;
    let err = Config::parse(Path::new("test.toml"), text).expect_err("zero line cap");

    assert!(matches!(err, ConfigError::Validation { .. }));
    assert!(format!("{err}").contains("line_cap_bytes must be greater than 0"));
}

#[test]
fn reports_io_error_for_missing_file() {
    let path = Path::new("artifacts/tests/config/does-not-exist.toml");
    let err = load(path).expect_err("missing file");

    assert!(matches!(err, ConfigError::Read { .. }));
}

#[test]
fn explicit_path_bypasses_default_resolution() {
    let path = Path::new("artifacts/tests/config/explicit-missing.toml");
    let err = load_from(Some(path)).expect_err("missing explicit path");

    assert!(matches!(err, ConfigError::Read { .. }));
    assert!(format!("{err}").contains("explicit-missing.toml"));
}

#[test]
fn reports_toml_syntax_error_with_location() {
    let text = "[general\nline_cap_bytes = 1024\n";
    let err = Config::parse(Path::new("test.toml"), text).expect_err("bad TOML");

    assert!(matches!(err, ConfigError::Parse { .. }));
    let message = format!("{err}");
    assert!(message.contains("line 1"), "{message}");
}

#[test]
fn default_paths_start_with_system_config() {
    let paths = default_paths();

    assert_eq!(paths[0], PathBuf::from(SYSTEM_CONFIG_PATH));
}
