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
- No history replay at startup and no persistence across restart.
- Main loop never calls D-Bus. A slow, hung, or absent notification service must
  not stop log tailing.
- Small memory footprint. Notification queues are bounded per source.
- Config and regex errors are loud startup failures.
- Patterns and notification templates are data. Adding or removing a source must
  not require code changes.

## 2. Context

```text
+-------------------+       +------------------------------+
| log-notify.toml   |       | monitored log files          |
| sources, patterns |       | fail2ban, clamav, tracee,    |
| urgency, queue    |       | aide, auditd, ...            |
+---------+---------+       +---------------+--------------+
          |                                 |
          | load + validate                 | inotify
          v                                 v
+----------------------------------------------------------+
|                     main thread                          |
|                                                          |
|  config.rs   ->  tail.rs  ->  match/template  ->         |
|                 (offset,     (regex, named     cooldown  |
|                  partial,     captures)            |     |
|                  rotation)                         v     |
|                                         bounded event queue
+----------------------------------------------------------+
                                                   |
                                                   | worker pops
                                                   v
+----------------------------------------------------------+
|                  notifier worker thread                  |
|                                                          |
|  retry/backoff  ->  zbus 6.5  ->  D-Bus Notifications    |
|       |                                                  |
|       v                                                  |
|  dropped-event log  (default /var/log/log-notify.log)    |
+----------------------------------------------------------+
          |
          v
   stderr event log
```

The daemon is one process with a synchronous main loop and one notifier worker.
There is no async runtime, no database, and no network listener.

## 3. Components

| Module | Responsibility | Does not |
|--------|----------------|----------|
| `main.rs` | CLI parsing, config path resolution, startup order, lockfile, signals, main event loop wiring, exit codes | Parse TOML, read logs, call D-Bus |
| `config.rs` | TOML deserialization, schema validation, regex compilation, template validation, queue capacity validation, precise errors | Watch files |
| `tail.rs` | Open sources at EOF, inotify watches, read complete lines, partial buffers, line cap, offset and inode tracking, rotation recovery | Match patterns or notify |
| `notify.rs` | Bounded queue, notifier worker, retry/backoff, zbus 6.5 backend, dry-run writer, dropped-event log | Read logs or decide cooldown |
| `cooldown.rs` | Per-source monotonic suppression windows | Send notifications |

The notifier is a trait so tests can use a fake without D-Bus. `--dry-run` is a
second implementation that prints events to stdout and never starts the worker.

## 4. Startup sequence

1. Parse CLI. Unknown flags are usage errors.
2. Resolve config path: explicit `--config`, else
   `/etc/log-notify/log-notify.toml`, else
   `~/.config/log-notify/log-notify.toml`.
3. Read and parse TOML.
4. Validate schema, queue capacities (`1..=100`), and compile every regex. On
   failure: print file, line, and reason to stderr; exit non-zero. Do not start
   watchers.
5. Acquire the advisory lock at `$XDG_RUNTIME_DIR/log-notify.lock`. If
   `$XDG_RUNTIME_DIR` is unset, use `/run/user/<uid>` when it exists; otherwise
   exit with a clear error. A held lock makes the second instance exit non-zero.
6. Open the dropped-event log at `general.dropped_log`, default
   `/var/log/log-notify.log`. On failure, warn once on stderr and use stderr as
   the dropped-event sink. This is not fatal.
7. Open each enabled source and stat it. Record dev/inode and set offset to the
   current size.
   - Missing path: warn, watch the nearest existing ancestor directory for
     creation, and continue. When an intermediate directory appears, re-resolve
     the path and watch the next existing ancestor until the source file exists.
     An ancestor that cannot be watched is a warning, not a crash.
   - Permission denied or another read error: exit non-zero with the path.
8. Start the notifier worker and add inotify watches.
9. Enter the event loop. Install SIGTERM/SIGINT handlers before or during
   startup so a fast service stop does not hang.

Config validation is a pure function of file content. This keeps SIGHUP reload
possible in v0.2 without redesigning the loader.

## 5. Main event loop

Blocking `read` on the inotify descriptor; no timer tick.

For each inotify event:

1. Map the watch descriptor to a source.
2. On `MODIFY`, `CLOSE_WRITE`, or `CREATE`: read from the current offset. For a
   missing source, `CREATE` re-resolves the path and attaches the next existing
   ancestor watch; when the source exists, open it at its current end.
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
- Convert bytes to text lossily (`String::from_utf8_lossy`) before regex
  matching. Invalid UTF-8 must not panic; replacement characters are matched as
  ordinary text.

Matching and enqueue:

1. Apply the compiled regex to the line.
2. If no match, continue.
3. Build summary and body by substituting `{line}` and named captures.
4. Ask the source cooldown whether this event is allowed.
5. If allowed, enqueue the event. The main loop never waits for D-Bus.
6. If suppressed by cooldown, do not enqueue. This is intentional, not a drop.
7. If the queue is full, the queue implementation drops the oldest event for that
   source, logs it, and accepts the new event.

## 6. Tail state

Per source:

```text
path            configured path
fd              open file descriptor (optional while missing)
dev, ino        identity of the open file
offset          next unread byte
partial         bytes after the last newline, capped
discarding      true while skipping the tail of an over-cap line
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
- Window starts at the successful enqueue, not at the D-Bus call. This keeps the
  main loop fast and makes bursts deterministic.
- Suppressed matches are discarded, not queued.
- `--once` exits after the first regex match has been processed. `--dry-run`
  prints it and exits; normal mode waits for the worker's delivery outcome
  (sent or dropped) before exiting.

## 8. Notification queue and worker

### Queue

- One global FIFO `VecDeque<Event>` behind a `Mutex`, with a `Condvar`.
- Per-source counters enforce `queue_capacity`.
- Default capacity: `10` per source. Allowed range: `1..=100`.
- Memory bound: `sum(queue_capacity) * line_cap_bytes`, plus string overhead.
- Overflow: remove the oldest event of the same source, write it to the
  dropped-event log with reason `queue_full`, push the new event.
- Enqueue is a short critical section. The worker pops under the same lock and
  sends outside the lock.
- Shutdown wakes the worker and drains or abandons pending events according to
  the signal path; no D-Bus call may block shutdown indefinitely.

### Worker

- Dedicated thread, one zbus 6.5 connection, `zbus::block_on` around each async
  call. No async runtime in the daemon.
- `ConnectionBuilder::method_timeout` is 5 seconds.
- Call `org.freedesktop.Notifications.Notify` with:
  - `app_name = "log-notify"`
  - `replaces_id = 0` in v0.1; per-source reuse is v0.2 polish
  - `app_icon = ""`
  - `summary` and `body` from the source templates
  - no actions
  - `urgency` hint: `low = 0`, `normal = 1`, `critical = 2`
  - `expire_timeout`: `-1` for low/normal, `0` for critical (never expire)
- Transient failure: retry up to 3 times with 1 s, 2 s, and 4 s backoff.
  Reconnect the zbus connection before a retry when the error indicates a lost
  connection.
- Missing service (`ServiceUnknown`, no session bus): no retry; drop the event
  and log it with reason `no_service`.
- Timeout or exhausted retries: drop the event and log it with reason
  `notify_failed`.
- A retry can produce a duplicate if the first call succeeded but the reply was
  lost. This is accepted.
- `--dry-run`: no worker, no queue; print the event to stdout immediately.

### Dropped-event log

- Path: `general.dropped_log`, default `/var/log/log-notify.log`.
- One line per drop: timestamp, source path, reason, summary, body.
- Newlines and control characters in the body are escaped so one event stays on
  one line.
- If the file cannot be opened or written, warn once on stderr and use stderr.
  A failed diagnostic write never stops tailing.
- No internal rotation. Use external `logrotate` with copytruncate, or restart
  the daemon after rotation. Reopening on SIGHUP is a v0.2 candidate.

## 9. Failure modes and decisions

| Situation | Behavior | Status |
|-----------|----------|--------|
| Invalid TOML, schema, regex, or queue capacity | Exit non-zero with file, line, reason | Decided |
| Config file missing | Exit non-zero with the attempted paths | Decided |
| No enabled sources | Config error | Decided |
| Log file missing at startup | Warn, watch nearest existing ancestor, continue | Decided |
| Log file unreadable (permissions) | Exit non-zero, precise path | Decided |
| Non-UTF-8 bytes | Lossy conversion per line | Decided |
| `$XDG_RUNTIME_DIR` unset | `/run/user/<uid>` if present, else exit | Decided |
| Line longer than cap | Keep cap, discard rest, match once at newline | Decided |
| File shrinks | Reset offset to 0 | Decided |
| Inode changes | Reopen at current end | Decided |
| Queue full | Drop oldest of same source, log `queue_full` | Decided |
| Notification service absent | Drop, log `no_service`, keep tailing | Decided |
| Transient D-Bus failure | Retry 1/2/4 s, then drop and log | Decided |
| Dropped log unwritable | Warn once, fall back to stderr | Decided |
| Second instance | Exit non-zero | Decided |
| SIGTERM / SIGINT | Close watchers, stop worker, exit 0 | Decided |
| SIGHUP | Ignored in v0.1; reload in v0.2 | Deferred |

There are no open behavior questions for v0.1.

## 10. Security and privacy

- The config is trusted input; it is not accepted from the network.
- Monitored logs are read-only. The daemon never writes to them.
- Notification bodies contain raw log lines. They may contain usernames, IP
  addresses, paths, or other sensitive data. Templates can change the body, but
  the default `body = "{line}"` displays the line as-is.
- The dropped-event log also contains raw log lines. Protect it with file
  permissions; `/var/log/log-notify.log` is often not writable by a user-session
  service, so the service unit must grant access or configure a user-writable
  path.
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
| Unit | Queue capacity, overflow drop, retry/backoff policy, dropped-log formatting | `cargo test` with fake clock/notifier |
| Integration | Append fixture lines, run `--dry-run`, assert stdout events | `cargo test` + fixtures |
| Integration | Rotation: copytruncate and inode replacement | temp files |
| Integration | Failing notifier: drops logged, tailing continues | fake notifier |
| Manual | Real log sources and a real notification daemon | `--dry-run` first, then live |

CI does not need a notification daemon. The notifier trait and `--dry-run` keep
tests deterministic. CI runs `cargo fmt --check`, `cargo clippy -- -D warnings`,
`cargo test`, and `cargo build --release` in a container.

## 12. Observability

- One stderr line per event: source path, summary, and outcome.
- One stderr line per notification failure, including the D-Bus error.
- Dropped events go to the dropped-event log with a reason.
- `--verbose` adds tail-debug lines: open, reopen, offset reset, inode change,
  suppressed matches, queue depth, retry attempts.
- Exit codes: 0 for clean shutdown and `--once` success; non-zero for config,
  lock, and startup errors.
- Service managers may swallow exit codes, so stderr and the dropped-event log
  must be truthful and sufficient on their own.

## 13. Future work

- v0.2: SIGHUP reload with validation; keep the old config on failure.
- v0.2: per-source `replaces_id` reuse to avoid stacked windows.
- v0.2: reopen the dropped log on SIGHUP or rotation.
- v0.2: packaging polish.
- Downstream: FeralOS aports package and user dinit unit.
- Not planned: durable spool or replay.
