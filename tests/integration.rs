//! End-to-end fixture checks for the shipped default config.
#![allow(clippy::expect_used, clippy::panic)]

use std::path::Path;

use log_notify::config;

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(path)
        .expect("read fixture")
        .trim_end_matches('\n')
        .to_owned()
}

#[test]
fn default_config_matches_all_shipped_sources() {
    let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/log-notify.toml");
    let config = config::load(&config_path).expect("load default config");

    let cases = [
        ("/var/log/fail2ban.log", fixture("fail2ban.txt")),
        ("/var/log/clamav/clamd.log", fixture("clamd.txt")),
        ("/var/log/dinit/tracee.log", fixture("tracee.txt")),
        ("/var/log/aide/aide.log", fixture("aide.txt")),
        ("/var/log/audit/audit.log", fixture("audit.txt")),
    ];

    for (source_path, line) in cases {
        let source = config
            .sources
            .iter()
            .find(|source| source.path.as_path() == Path::new(source_path))
            .expect("source in default config");
        let captures = source
            .compiled
            .captures(&line)
            .unwrap_or_else(|| panic!("{source_path} did not match line: {line}"));
        let rendered = source.compiled.render(&line, &captures);

        assert!(!rendered.summary.is_empty(), "{source_path}");
        assert_eq!(rendered.body, line, "{source_path}");
    }
}

#[test]
fn non_matching_line_returns_none() {
    let config_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("config/log-notify.toml");
    let config = config::load(&config_path).expect("load default config");
    let source = config
        .sources
        .iter()
        .find(|source| source.path.as_path() == Path::new("/var/log/clamav/clamd.log"))
        .expect("clamav source");

    assert!(source.compiled.captures("ordinary log line").is_none());
}
