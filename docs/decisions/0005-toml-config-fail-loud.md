# ADR-0005: TOML config with fail-loud validation

## Status

Accepted — 2026-11-26. Deciders: project owner, implementation agent.

## Context

Users must declare which logs to read and how to recognize events. The config is
edited by hand, often on a security-sensitive host. A silent typo or invalid
regex means missed notifications, which is the worst failure mode for this tool.
The source spec fixes TOML and per-source Rust regex patterns.

## Decision

We will use TOML parsed with `serde`. At startup the daemon:

- validates the schema, required fields, urgency values, and source count;
- compiles every regex;
- validates template placeholders;
- exits non-zero on any error with the config file, line, and reason.

Path resolution is `--config`, then `/etc/log-notify/log-notify.toml`, then
`~/.config/log-notify/log-notify.toml`. The schema has `[general]` with
`line_cap_bytes` (default 8192) and repeated `[[source]]` tables with `path`,
`pattern`, `summary`, optional `body`, `urgency`, `cooldown_secs`, and `enabled`.

## Options

- **TOML + serde + startup validation** — Selected. Readable, typed, familiar to
  Rust and Linux users, precise errors.
- **YAML** — Rejected. More indentation traps and a larger parser surface.
- **JSON** — Rejected. No comments, poor hand-editing ergonomics.
- **CLI flags only** — Rejected. N sources with regexes and templates do not fit
  a command line.
- **Lazy validation on first match** — Rejected. It hides errors until an event
  happens, which defeats the fail-loud requirement.

## Consequences

- Misconfiguration is caught at startup, before any watcher runs.
- Config changes require a restart in v0.1; SIGHUP reload is ADR-0007.
- Unknown fields should be rejected to catch typos; this is stricter than
  permissive parsing and is the right default for a notifier.
- The shipped default config is a real deployment artifact, so pattern changes
  need evidence from real logs.
