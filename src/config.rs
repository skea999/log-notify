//! Configuration loading and validation.
//!
//! TOML sources, queue capacities, and the dropped-event log are validated at
//! startup. Errors carry the config file, line, and reason.

use std::ops::Range;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;
use toml::Spanned;

use crate::pattern::{CompiledPattern, PatternError};

/// Default maximum bytes considered per log line.
pub const DEFAULT_LINE_CAP_BYTES: usize = 8192;
/// Default dropped-event log path.
pub const DEFAULT_DROPPED_LOG: &str = "/var/log/log-notify.log";
/// Default pending notification capacity per source.
pub const DEFAULT_QUEUE_CAPACITY: usize = 10;
/// Maximum pending notification capacity per source.
pub const MAX_QUEUE_CAPACITY: usize = 100;
/// System-wide config path.
pub const SYSTEM_CONFIG_PATH: &str = "/etc/log-notify/log-notify.toml";

/// Fully validated runtime configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Global settings.
    pub general: General,
    /// Configured log sources, in file order.
    pub sources: Vec<Source>,
}

/// Global settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct General {
    /// Maximum bytes considered per line.
    pub line_cap_bytes: usize,
    /// File for undeliverable and overflow events.
    pub dropped_log: PathBuf,
}

/// One configured log source.
#[derive(Debug, Clone)]
pub struct Source {
    /// Log file path.
    pub path: PathBuf,
    /// Rust `regex` pattern source.
    pub pattern: String,
    /// Notification title template source.
    pub summary: String,
    /// Notification body template source.
    pub body: String,
    /// Compiled regex and templates.
    pub compiled: CompiledPattern,
    /// Notification urgency.
    pub urgency: Urgency,
    /// Per-source suppression window after enqueue.
    pub cooldown_secs: u64,
    /// Maximum pending notifications for this source.
    pub queue_capacity: usize,
    /// Whether the source is tailed.
    pub enabled: bool,
}

/// Notification urgency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Urgency {
    /// Low importance.
    Low,
    /// Normal importance.
    #[default]
    Normal,
    /// Critical importance; never expires.
    Critical,
}

/// Configuration load or validation failure.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The config file could not be read.
    #[error("cannot read config file {path}: {source}")]
    Read {
        /// Config path.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// The TOML syntax or schema is invalid.
    #[error("invalid TOML in {path}: {source}")]
    Parse {
        /// Config path.
        path: PathBuf,
        /// TOML parser error with line and column.
        #[source]
        source: toml::de::Error,
    },
    /// A semantic value is invalid.
    #[error("{path}:{line}:{column}: {message}")]
    Validation {
        /// Config path.
        path: PathBuf,
        /// 1-based line.
        line: usize,
        /// 1-based column.
        column: usize,
        /// Human-readable reason.
        message: String,
    },
    /// No default config file exists.
    #[error("no config file found; tried: {tried}")]
    NotFound {
        /// Comma-separated attempted paths.
        tried: String,
    },
}

impl Config {
    /// Parse and validate config text.
    pub fn parse(path: &Path, text: &str) -> Result<Self, ConfigError> {
        let raw: RawConfig = toml::from_str(text).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_raw(path, text, raw)
    }

    fn from_raw(path: &Path, text: &str, raw: RawConfig) -> Result<Self, ConfigError> {
        let general = raw.general.unwrap_or_default();

        let line_cap_bytes = match general.line_cap_bytes {
            Some(spanned) => {
                let span = spanned.span();
                let value = spanned.into_inner();
                if value == 0 {
                    return Err(validation(
                        path,
                        text,
                        span,
                        "general.line_cap_bytes must be greater than 0".to_owned(),
                    ));
                }
                value
            }
            None => DEFAULT_LINE_CAP_BYTES,
        };

        let dropped_log = match general.dropped_log {
            Some(spanned) => {
                let span = spanned.span();
                let value = spanned.into_inner();
                if value.trim().is_empty() {
                    return Err(validation(
                        path,
                        text,
                        span,
                        "general.dropped_log must not be empty".to_owned(),
                    ));
                }
                PathBuf::from(value)
            }
            None => PathBuf::from(DEFAULT_DROPPED_LOG),
        };

        if raw.sources.is_empty() {
            return Err(validation(
                path,
                text,
                0..0,
                "at least one [[source]] is required".to_owned(),
            ));
        }

        let mut sources = Vec::with_capacity(raw.sources.len());
        for (index, raw_source) in raw.sources.into_iter().enumerate() {
            let number = index + 1;
            let source_path = required_text(path, text, number, "path", raw_source.path, true)?;
            let pattern_span = raw_source.pattern.span();
            let pattern_text =
                required_text(path, text, number, "pattern", raw_source.pattern, false)?;
            let summary_span = raw_source.summary.span();
            let summary_text =
                required_text(path, text, number, "summary", raw_source.summary, true)?;
            let body_span = raw_source.body.as_ref().map(Spanned::span);
            let body_text = raw_source
                .body
                .map_or_else(String::new, Spanned::into_inner);
            let compiled = match CompiledPattern::compile(&pattern_text, &summary_text, &body_text)
            {
                Ok(compiled) => compiled,
                Err(PatternError::Regex(source)) => {
                    return Err(validation(
                        path,
                        text,
                        pattern_span,
                        format!("pattern: {source}"),
                    ));
                }
                Err(PatternError::ReservedCapture { name }) => {
                    return Err(validation(
                        path,
                        text,
                        pattern_span,
                        format!("pattern: `{name}` is a reserved capture name"),
                    ));
                }
                Err(PatternError::Template { field, message }) => {
                    let span = if field == "summary" {
                        summary_span
                    } else {
                        body_span.unwrap_or(0..0)
                    };
                    return Err(validation(path, text, span, format!("{field}: {message}")));
                }
            };
            let urgency = raw_source
                .urgency
                .map_or(Urgency::Normal, Spanned::into_inner);
            let cooldown_secs = raw_source.cooldown_secs.map_or(0, Spanned::into_inner);
            let queue_capacity = match raw_source.queue_capacity {
                Some(spanned) => {
                    let span = spanned.span();
                    let value = spanned.into_inner();
                    if !(1..=MAX_QUEUE_CAPACITY).contains(&value) {
                        return Err(validation(
                            path,
                            text,
                            span,
                            format!(
                                "source {number}: queue_capacity must be 1..={MAX_QUEUE_CAPACITY}"
                            ),
                        ));
                    }
                    value
                }
                None => DEFAULT_QUEUE_CAPACITY,
            };
            let enabled = raw_source.enabled.is_none_or(Spanned::into_inner);

            sources.push(Source {
                path: PathBuf::from(source_path),
                pattern: pattern_text,
                summary: summary_text,
                body: body_text,
                compiled,
                urgency,
                cooldown_secs,
                queue_capacity,
                enabled,
            });
        }

        Ok(Self {
            general: General {
                line_cap_bytes,
                dropped_log,
            },
            sources,
        })
    }
}

/// Load and validate a config file.
pub fn load(path: &Path) -> Result<Config, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    Config::parse(path, &text)
}

/// Paths tried when `--config` is not given.
#[must_use]
pub fn default_paths() -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from(SYSTEM_CONFIG_PATH)];
    if let Some(home) = std::env::var_os("HOME") {
        paths.push(PathBuf::from(home).join(".config/log-notify/log-notify.toml"));
    }
    paths
}

/// Resolve the first existing default config path.
pub fn resolve_default_path() -> Result<PathBuf, ConfigError> {
    let tried = default_paths();
    if let Some(path) = tried.iter().find(|path| path.is_file()) {
        return Ok(path.clone());
    }
    let tried = tried
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(ConfigError::NotFound { tried })
}

/// Load a config chosen by the caller, or resolve the defaults.
///
/// An explicit path is used as-is; if it is missing, the error names that path.
pub fn load_from(explicit: Option<&Path>) -> Result<Config, ConfigError> {
    explicit.map_or_else(load_default, load)
}

/// Resolve and load the default config.
pub fn load_default() -> Result<Config, ConfigError> {
    let path = resolve_default_path()?;
    load(&path)
}

fn required_text(
    path: &Path,
    text: &str,
    source_number: usize,
    field: &str,
    spanned: Spanned<String>,
    trim: bool,
) -> Result<String, ConfigError> {
    let span = spanned.span();
    let value = spanned.into_inner();
    let empty = if trim {
        value.trim().is_empty()
    } else {
        value.is_empty()
    };
    if empty {
        return Err(validation(
            path,
            text,
            span,
            format!("source {source_number}: {field} must not be empty"),
        ));
    }
    Ok(value)
}

fn validation(path: &Path, text: &str, span: Range<usize>, message: String) -> ConfigError {
    let (line, column) = line_col(text, span);
    ConfigError::Validation {
        path: path.to_path_buf(),
        line,
        column,
        message,
    }
}

fn line_col(text: &str, span: Range<usize>) -> (usize, usize) {
    let offset = span.start.min(text.len());
    let before = text.get(..offset).unwrap_or(text);
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .map_or(1, |line| line.chars().count() + 1);
    (line, column)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(default)]
    general: Option<RawGeneral>,
    #[serde(default, rename = "source")]
    sources: Vec<RawSource>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawGeneral {
    #[serde(default)]
    line_cap_bytes: Option<Spanned<usize>>,
    #[serde(default)]
    dropped_log: Option<Spanned<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSource {
    path: Spanned<String>,
    pattern: Spanned<String>,
    summary: Spanned<String>,
    body: Option<Spanned<String>>,
    urgency: Option<Spanned<Urgency>>,
    cooldown_secs: Option<Spanned<u64>>,
    queue_capacity: Option<Spanned<usize>>,
    enabled: Option<Spanned<bool>>,
}
