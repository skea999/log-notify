# log-notify

Tail configured log files, match regular expressions, and emit freedesktop
desktop notifications.

`log-notify` is a small Linux daemon for a user session. It is a notifier, not a
security control: it reads logs, reports matches, and keeps running when the
desktop notification service is absent.

**Status:** documentation, CI scaffold, and Rust scaffold initialized on
2026-11-26. The daemon behavior is not implemented yet. The contract below is
defined by [`docs/log-notify-spec.md`](docs/log-notify-spec.md) and planned in
[`tasks/plan.md`](tasks/plan.md).

## Overview

Security tools already write useful events to their own logs. Wiring a native
hook into every tool is fragile and non-uniform. `log-notify` uses one uniform
input path instead: read log files, apply per-source regex patterns, and raise a
desktop notification for each match.

- Linux only. The daemon uses inotify and does not poll.
- Wayland and X11 agnostic. Notifications go through the D-Bus
  `org.freedesktop.Notifications` service.
- Generic tool. The shipped default config targets FeralOS security-stack log
  paths; other deployments replace it with their own sources.
- Read-only. It never writes to the monitored logs and never changes the tools
  that produce them.

## How it works

```text
configured log files
        |
        v
  inotify tailer  ->  complete-line buffer  ->  per-source regex
                                                   |
                                                   v
                                            per-source cooldown
                                                   |
                                                   v
                                   bounded per-source notification queue
                                                   |
                                                   v
                                         notifier worker thread
                                                   |
                                                   v
                                     D-Bus Notifications.Notify
```

At startup each source is opened and its offset is placed at the current end of
file. History is not replayed. New bytes are read as complete lines; a trailing
partial line is buffered until its newline arrives. Lines longer than the
configured cap are truncated before matching. The main loop never calls D-Bus.

## Requirements

- Linux with inotify.
- A running D-Bus session and a freedesktop notification daemon for visible
  notifications. Without one, events are retried, then dropped and logged;
  tailing continues.
- Read permission on every configured log file.
- Rust stable (build from source). The release plan also targets
  `x86_64-unknown-linux-musl` for a static binary.

## Build

```bash
cargo build --release
./target/release/log-notify --version
```

Static musl build (planned release target):

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

## Install

From a checkout:

```bash
cargo install --path .
```

Or install the release binary and the shipped config manually:

```bash
install -Dm755 target/release/log-notify ~/.local/bin/log-notify
install -Dm644 config/log-notify.toml ~/.config/log-notify/log-notify.toml
```

The default config is installed to `/etc/log-notify/log-notify.toml` on
FeralOS. Use `--config` to point at another file.

## Usage

```text
log-notify [--config PATH] [--dry-run] [--once] [--verbose] [--version]
```

| Flag | Behavior |
|------|----------|
| `--config PATH` | Use this config file. Without it, try `/etc/log-notify/log-notify.toml`, then `~/.config/log-notify/log-notify.toml`. |
| `--dry-run` | Run normally, but print matched events to stdout instead of sending notifications. Useful for pattern testing. |
| `--once` | Exit after the first match. In normal mode, wait for its delivery outcome before exiting. Useful for scripts and tests. |
| `--verbose` | Add tail-debug lines to stderr. |
| `--version` | Print the version and exit. |

Examples:

```bash
# Test patterns against a fixture log without notifying
log-notify --config ./config/log-notify.toml --dry-run --verbose

# One-shot check
log-notify --config ./config/log-notify.toml --dry-run --once

# Normal session service
log-notify --config ~/.config/log-notify/log-notify.toml
```

## Configuration

The config is TOML. Regexes are compiled and validated at startup. A bad config
is a loud startup failure: the process exits non-zero with the file, line, and
reason. Silent misconfiguration is the worst failure mode for a notifier.

### Default config

The repository ships `config/log-notify.toml` with the FeralOS security-stack
sources. The config is data: add, remove, or edit sources in TOML without
changing code. The supplied patterns were validated against the target logs; if
a log format changes later, fix only the config file.

```toml
# /etc/log-notify/log-notify.toml

[general]
line_cap_bytes = 8192
dropped_log = "/var/log/log-notify.log"

[[source]]
path = "/var/log/fail2ban.log"
pattern = ' Ban [0-9]+\.[0-9]+\.[0-9]+\.[0-9]+ '
summary = "fail2ban ban"
body    = "{line}"
urgency = "normal"
cooldown_secs = 60
queue_capacity = 10

[[source]]
path = "/var/log/clamav/clamd.log"
pattern = ' FOUND$'
summary = "clamav detection"
body    = "{line}"
urgency = "critical"
cooldown_secs = 0

[[source]]
path = "/var/log/dinit/tracee.log"
# FeralOS default source; edit or remove without code changes
pattern = '"signature"'
summary = "tracee detection"
body    = "{line}"
urgency = "critical"
cooldown_secs = 30

[[source]]
path = "/var/log/aide/aide.log"
pattern = '^(added|removed|changed): '
summary = "aide file change"
body    = "{line}"
urgency = "normal"
cooldown_secs = 0

[[source]]
path = "/var/log/audit/audit.log"
pattern = ' type=\S+ msg=.*key="(identity|sudoers|time-change|modules|system_init|sshd_config)"'
summary = "auditd watch hit"
body    = "{line}"
urgency = "critical"
cooldown_secs = 30
```

### Source fields

| Field | Required | Default | Description |
|-------|----------|---------|-------------|
| `path` | yes | — | Log file to tail. |
| `pattern` | yes | — | Rust `regex` syntax. Compiled at startup; an invalid regex is a config error. |
| `summary` | yes | — | Notification title. Supports `{line}` and named capture groups such as `{name}`. |
| `body` | no | empty | Notification body. Supports the same placeholders as `summary`. |
| `urgency` | no | `normal` | `low`, `normal`, or `critical`. |
| `cooldown_secs` | no | `0` | After a notification, suppress further matches from the same source for this many seconds. `0` disables suppression. |
| `queue_capacity` | no | `10` | Maximum pending notifications for this source, `1..=100`. Overflow drops the oldest event for that source and logs it. |
| `enabled` | no | `true` | Set to false to keep the source in the file but stop tailing it. |

General fields:

| Field | Default | Description |
|-------|---------|-------------|
| `general.line_cap_bytes` | `8192` | Maximum bytes per line. Longer lines are capped before matching. |
| `general.dropped_log` | `/var/log/log-notify.log` | File for dropped events. If unwritable, the daemon logs one warning to stderr and falls back to stderr. |

### Notification urgency

| Config value | D-Bus urgency hint | Expiry |
|--------------|--------------------|--------|
| `low` | `0` | daemon default (`expire_timeout = -1`) |
| `normal` | `1` | daemon default (`expire_timeout = -1`) |
| `critical` | `2` | never expire (`expire_timeout = 0`) |

The notification `app_name` is `log-notify`. v0.1 uses `replaces_id = 0`; per-source
reuse so a burst does not stack windows is v0.2 polish.

## Runtime behavior

- **Start at end of file.** No history replay at startup.
- **Complete lines only.** A partial trailing line stays buffered until `\n`
  arrives. When a line exceeds the cap, the first `line_cap_bytes` are kept, the
  rest is discarded until the newline, and the capped prefix is matched once.
- **Rotation.** If the file shrinks below the last offset (copytruncate), reset
  the offset to 0 and read the new content. If the inode changes, reopen the path
  from its current end.
- **Cooldown per source.** After a notification, suppress further matches from
  that source for its configured window. Default is `60` seconds in the shipped
  config; the schema default is `0`.
- **Bounded queue.** The main loop enqueues events; a dedicated worker sends them.
  Each source holds `10` events by default, maximum `100`. When full, the oldest
  event for that source is dropped and written to the dropped-event log.
- **Notification failures.** The worker retries transient D-Bus errors up to 3
  times with 1 s, 2 s, and 4 s backoff. Each call has a 5 s timeout. A missing
  service or exhausted retries drops the event and logs it. Tailing never waits
  for D-Bus.
- **Dropped events.** Written to `general.dropped_log`, default
  `/var/log/log-notify.log`, one line each. If the file is not writable, the
  daemon warns once and uses stderr.
- **No replay or persistence.** No cursor files, no unread counters, no durable
  spool. Pending in-memory events are lost on restart. The logs themselves are
  the record.
- **Single instance.** An advisory lock at
  `$XDG_RUNTIME_DIR/log-notify.lock`; a second instance exits with an error.
- **Shutdown.** SIGTERM or SIGINT closes watchers and exits 0.
- **Config reload.** SIGHUP reload is deferred to v0.2. v0.1 requires a restart
  after a config change.
- **Logging.** One truthful line per event to stderr. `--verbose` adds tail-debug
  lines. Dropped events also go to the dropped-event log. Do not rely on process
  exit codes for outcome signaling; service managers may swallow them.

## Compatibility

`log-notify` works with any freedesktop notification daemon. It does not depend
on a specific desktop environment, notification theme, or compositor.

## Non-goals

- No notification replay or persistence.
- No durable spool; the queue is in memory and bounded.
- No GUI.
- No log storage or indexing.
- No filtering beyond per-line regex.
- No multi-user routing; notifications go to the session that is up.
- No remote or syslog input. Files only.

## Development

Layout:

```text
Cargo.toml
rust-toolchain.toml
ci/                CI container image
scripts/           test, static, docs, duplicate, release gates
src/main.rs        CLI, startup, config load, fail-loud validation
src/config.rs      TOML parse and validation, regex compilation
src/tail.rs        inotify tailer: offsets, partial lines, shrink reset
src/notify.rs      queue, notifier worker, dropped log, zbus backend
src/cooldown.rs    per-source rate limiting
tests/             smoke, pattern, tail, cooldown, config tests
config/            shipped default config
```

Quality gates run inside the CI container; the host toolchain is not used:

```bash
scripts/dev.sh image      # build log-notify-ci:ci
scripts/dev.sh check      # fmt + clippy-driven test gate
scripts/dev.sh static     # typos, shellcheck, shfmt, taplo, actionlint,
                         # zizmor, gitleaks, cargo-geiger, cargo-deny,
                         # cargo-audit, cargo-machete, jscpd, SBOM
scripts/dev.sh docs       # markdownlint-cli2
scripts/dev.sh verify     # static + docs + static-musl release package
```

CI runs the same sequence in a container. The release package is built with
`x86_64-unknown-linux-musl` and must have zero dynamic dependencies.

The notification backend is zbus 6.5 with a dedicated worker. The file-event
backend is the direct `inotify` crate, Linux only.

CI runs the same gates on `ubuntu-latest` with a pinned Rust toolchain. Release
builds are tag-driven and upload the release binary, including the musl target.

See [`tasks/todo.md`](tasks/todo.md) for the ordered implementation tasks and
[`docs/architecture.md`](docs/architecture.md) for the component design.

## Support

- Product contract: [`docs/log-notify-spec.md`](docs/log-notify-spec.md)
- Architecture: [`docs/architecture.md`](docs/architecture.md)
- Decisions: [`docs/decisions/`](docs/decisions/)
- Implementation plan: [`tasks/plan.md`](tasks/plan.md)
- Issue tracker: the repository issue tracker, once the GitHub remote exists.

## Contributing

- Conventional commits: `feat:`, `fix:`, `docs:`, `test:`, `refactor:`,
  `chore:`.
- Run `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` before
  committing.
- Update `CHANGELOG.md` for notable changes.
- Do not change shipped patterns without real log output as evidence.

## License

ISC. See [`LICENSE`](LICENSE).
