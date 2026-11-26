# log-notify — Architecture

Source contract: [`log-notify-spec.md`](log-notify-spec.md).
Decisions: [`decisions/`](decisions/). Plan: [`../tasks/plan.md`](../tasks/plan.md).

## 1. Purpose and constraints

`log-notify` is a Linux user-session daemon. It watches configured log files,
matches one regex per source, and emits freedesktop notifications through D-Bus.

Constraints that shape the design:

- Linux only; inotify event-driven, no polling.
- Wayland and X11 agnostic; notifications use `org.freedesktop.Notifications`.
- Read-only on monitored logs. No native tool hooks.
- No history replay at startup and no persistence.
- Must keep tailing when the notification service is absent.
- Small dependency tree and few MB RSS.
- Config and regex errors are loud startup failures.

## 2. Context

```text
+-------------------+       +------------------------------+
| log-notify.toml   |       | monitored log files          |
| sources, patterns |       | fail2ban, clamav, tracee,    |
| urgency, cooldown |       | aide, auditd, ...            |
+---------+---------+       +---------------+--------------+
          |                                 |
          | load + validate                 | inotify
          v                                 v
+----------------------------------------------------------+
|                     log-notify daemon                    |
|                                                          |
|  config.rs   ->  tail.rs  ->  match/template  ->         |
|                 (offset,     (regex, named     cooldown  |
|                  partial,     captures)         |        |
|                  rotation)                      v        |
|                                            notify.rs     |
|                                                |         |
|                                                v         |
|                                     D-Bus Notifications  |
+----------------------------------------------------------+
                           |
                           v
                    stderr event log
```

The daemon is a single process with a single event loop. There is no database,
no network listener, and no on-disk event state.

## 3. Components

| Module | Responsibility | Does not |
|--------|----------------|----------|
| `main.rs` | CLI parsing, config path resolution, startup order, lockfile, signals, event loop wiring, exit codes | Parse TOML or read logs directly |
| `config.rs` | TOML deserialization, schema validation, regex compilation, template validation, precise errors | Watch files |
| `tail.rs` | Open sources at EOF, inotify watches, read complete lines, partial buffers, line cap, offset and inode tracking, rotation recovery | Match patterns or notify |
| `notify.rs` | Notifier abstraction, dry-run writer, zbus D-Bus backend, urgency/expiry mapping, failure reporting | Decide whether an event is allowed (cooldown) |
| `cooldown.rs` | Per-source monotonic suppression windows | Send notifications |
| `main.rs` event loop | Order: read bytes → lines → match → cooldown → notify → log | — |

The notifier is a trait so tests can use a fake without D-Bus. `--dry-run` is a
second implementation that prints events to stdout.

## 4. Startup sequence

1. Parse CLI. Unknown flags are usage errors.
2. Resolve config path: explicit `--config`, else
   `/etc/log-notify/log-notify.toml`, else
   `~/.config/log-notify/log-notify.toml`.
3. Read and parse TOML.
4. Validate schema and compile every regex. On failure: print file, line, and
   reason to stderr; exit non-zero. Do not start watchers.
5. Acquire the advisory lock at `$XDG_RUNTIME_DIR/log-notify.lock`. If held,
   print an error and exit non-zero.
6. Open each enabled source and stat it. Record dev/inode and set offset to the
   current size. Missing or unreadable sources are a startup decision; see §9.
7. Add inotify watches.
8. Enter the event loop. Install SIGTERM/SIGINT handlers before or during
   startup so a fast service stop does not hang.

Config validation is a pure function of file content. This keeps SIGHUP reload
possible in v0.2 without redesigning the loader.

## 5. Event loop

Blocking `read` on the inotify descriptor; no timer tick.

For each inotify event:

1. Map the watch descriptor to a source.
2. On `MODIFY`, `CLOSE_WRITE`, or `CREATE`: read from the current offset.
3. Ignore events that do not advance the offset.
4. On shrink (`size < offset`): reset offset to 0 and read from the start of the
   new content. This covers copytruncate.
5. On inode change (`stat` dev/ino differs): close the old descriptor, reopen the
   path, seek to the current end, and continue. Do not replay the new file's
   history.
6. On delete or move: keep the source configured; log at verbose level. Reopen
   when the path appears again.

Reading:

- Read into a bounded buffer.
- Append to the source's partial-line buffer.
- Split on `\n`; keep the trailing partial bytes.
- If the partial buffer exceeds the line cap without a newline, keep the first
  `line_cap_bytes`, enter a discard-until-newline state, and match the capped
  prefix once when the newline arrives. This prevents unbounded memory growth
  and prevents multiple events from one long line.
- Convert bytes to text lossily before regex matching; invalid UTF-8 must not
  panic. Exact policy is an open question in the plan.

Matching and notification:

1. Apply the compiled regex to the line.
2. If no match, continue.
3. Build summary and body by substituting `{line}` and named captures.
4. Ask the source cooldown whether this event is allowed.
5. If allowed, call the notifier and record the emission time.
6. If suppressed, do not notify. Verbose logging may count suppressions.
7. If the notifier fails, write one stderr line and continue. No immediate retry.

## 6. Tail state

Per source:

```text
path            configured path
fd              open file descriptor (optional while missing)
dev, ino        identity of the open file
offset          next unread byte
partial         bytes after the last newline, capped
last_notified   monotonic instant of last emission, for cooldown
```

Invariants:

- `offset` never moves backwards except on detected shrink, where it resets to 0.
- `partial` is always shorter than or equal to the line cap.
- A line is matched only after its newline arrives. If the cap is hit first, the
  capped prefix is matched once when the newline arrives.
- No byte is matched twice because of a normal append.
- A replaced file does not cause a replay of its old content.

## 7. Cooldown

Per-source, monotonic clock, no persistence.

- Default schema value: `0` (disabled).
- Shipped config: `60` for fail2ban, `30` for tracee and auditd, `0` for clamav
  and aide.
- Window starts at the successful notification emission.
- Suppressed matches are discarded, not queued.
- `--once` exits after the first event that would be emitted, including a match
  that passes cooldown. With a fresh process, cooldown is empty, so the first
  match emits.

## 8. Notification backend

- `zbus` 5.x with the blocking API. Pin the major version for v0.1. zbus 6
  removed the blocking API; upgrading requires an async refactor (ADR-0004).
- Call `org.freedesktop.Notifications.Notify` with:
  - `app_name = "log-notify"`
  - `replaces_id = 0` in v0.1; per-source reuse is v0.2 polish
  - `app_icon = ""`
  - `summary` and `body` from the source templates
  - no actions
  - `urgency` hint: `low = 0`, `normal = 1`, `critical = 2`
  - `expire_timeout`: `-1` for low/normal, `0` for critical (never expire)
- Failure handling:
  - No session bus or no notification service: log one stderr line, drop the
    event, keep tailing.
  - Connection loss: mark the connection unusable, log, and attempt a lazy
    reconnect on the next event. Never loop retries.
  - `--dry-run`: print to stdout, never touch D-Bus.
- The notifier trait keeps D-Bus out of unit tests.

## 9. Failure modes and decisions

| Situation | Behavior | Status |
|-----------|----------|--------|
| Invalid TOML, schema, or regex | Exit non-zero with file, line, reason | Decided |
| Config file missing | Exit non-zero with the attempted paths | Decided |
| No enabled sources | Config error | Decided |
| Notification service absent | Drop, stderr, keep tailing | Decided |
| Log file missing at startup | Proposed: warn, watch parent for creation, continue | Open |
| Log file unreadable (permissions) | Proposed: exit non-zero, precise path | Open |
| Non-UTF-8 bytes | Proposed: lossy conversion per line | Open |
| `$XDG_RUNTIME_DIR` unset | Proposed: fallback to `/run/user/<uid>` or fail loud | Open |
| Line longer than cap | Truncate to cap before matching | Decided |
| File shrinks | Reset offset to 0 | Decided |
| Inode changes | Reopen at current end | Decided |
| Second instance | Exit non-zero | Decided |
| SIGTERM / SIGINT | Close watchers, exit 0 | Decided |
| SIGHUP | Ignored in v0.1; reload in v0.2 | Deferred |

Open items are tracked in [`../tasks/plan.md`](../tasks/plan.md).

## 10. Security and privacy

- The config is trusted input; it is not accepted from the network.
- Monitored logs are read-only. The daemon never writes to them.
- Notification bodies contain raw log lines. They may contain usernames, IP
  addresses, paths, or other sensitive data. Templates can change the body, but
  the default `body = "{line}"` displays the line as-is.
- No secrets belong in the config. There is no credential handling.
- Run as the session user with the minimum read permissions needed. Do not run
  the daemon as root just to read logs; grant group read access instead.
- Rust `regex` has linear-time matching; no backtracking blow-up from log input.
- The daemon does not open a network port.

## 11. Testing strategy

| Level | What | Tooling |
|-------|------|---------|
| Unit | Config parse and validation, regex compile, template substitution, cooldown clock, line cap | `cargo test` |
| Unit | Tail state transitions with temp files and injected events | `cargo test` |
| Integration | Append fixture lines, run `--dry-run`, assert stdout events | `cargo test` + fixtures |
| Integration | Rotation: copytruncate and inode replacement | temp files |
| Manual | Real log sources and a real notification daemon | `--dry-run` first, then live |

CI does not need a notification daemon. The notifier trait and `--dry-run` keep
tests deterministic. CI runs `cargo fmt --check`, `cargo clippy -- -D warnings`,
`cargo test`, and `cargo build --release`.

## 12. Observability

- One stderr line per event: source path, summary, and outcome.
- One stderr line per notification failure, including the D-Bus error.
- `--verbose` adds tail-debug lines: open, reopen, offset reset, inode change,
  suppressed matches.
- Exit codes: 0 for clean shutdown and `--once` success; non-zero for config,
  lock, and startup errors.
- Service managers may swallow exit codes, so stderr lines must be truthful and
  sufficient on their own.

## 13. Future work

- v0.2: SIGHUP reload with validation; keep the old config on failure.
- v0.2: per-source `replaces_id` reuse to avoid stacked windows.
- v0.2: packaging polish.
- Downstream: FeralOS aports package and user dinit unit.
