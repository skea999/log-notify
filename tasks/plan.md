# Implementation Plan: log-notify v0.1

Source spec: [`docs/log-notify-spec.md`](../docs/log-notify-spec.md)
Architecture: [`docs/architecture.md`](../docs/architecture.md)
Task list: [`todo.md`](todo.md)

## Overview

Build a single-binary Rust daemon that tails configured log files with inotify,
matches one regex per source, applies a per-source cooldown, and sends
freedesktop notifications through D-Bus. Start every source at end of file,
never replay history, survive rotation, and keep tailing when no notification
service exists.

Reliability is the priority. The main loop never calls D-Bus. A dedicated
notifier worker drains a small bounded per-source queue, retries transient
failures, and writes undeliverable events to a dropped-event log.

Documentation initialization is complete. No Rust code exists yet.

## Architecture Decisions

- [ADR-0002](../docs/decisions/0002-rust-single-binary-daemon.md): Rust stable,
  edition 2024, single binary, synchronous main loop, Linux only.
- [ADR-0003](../docs/decisions/0003-read-only-log-relay.md): read logs only, no
  native hooks, no replay or persistence.
- [ADR-0004](../docs/decisions/0004-zbus-notification-backend.md): zbus 6.5
  async API driven by `zbus::block_on` in a dedicated worker; 5 s method
  timeout; urgency mapping.
- [ADR-0005](../docs/decisions/0005-toml-config-fail-loud.md): TOML with serde;
  validate schema, regexes, queue capacities, and templates at startup; fail
  loud.
- [ADR-0006](../docs/decisions/0006-keep-isc-license.md): ISC license.
- [ADR-0007](../docs/decisions/0007-defer-sighup-reload.md): SIGHUP reload in
  v0.2; v0.1 requires restart.
- [ADR-0008](../docs/decisions/0008-bounded-notification-queue.md): bounded
  in-memory per-source queue, dropped-event log, bounded retries.

Proposed dependency set, to confirm at scaffold time with Context7 and
`cargo tree`:

| Purpose | Crate | Notes |
|---------|-------|-------|
| CLI | `clap` 4.x derive | Version, help, flags. |
| Config | `serde`, `toml` 1.x | Reject unknown fields; use `toml::Spanned` for line/column in semantic errors. |
| Matching | `regex` 1.x | Linear-time engine; compile at startup. |
| File events | `inotify` 0.11.x | Direct, Linux-only, precise control. |
| D-Bus | `zbus` 6.5.x | Async API; `zbus::block_on` in the worker; `method_timeout`. |
| Signals | `signal-hook` 0.4.x | SIGTERM, SIGINT; SIGHUP ignored. |
| Lock | `rustix` | Advisory `flock` on the runtime path. |
| Timestamps | `time` | Dropped-event log timestamps. |
| Errors | `anyhow` or `thiserror` | Typed config errors with file/line context. |

No async runtime. No network dependency.

## Task List

### Phase 1: Foundation

- [x] T1: Cargo scaffold, pinned toolchain, module skeleton
- [x] T2: Containerized CI gates

**Checkpoint: Foundation**
- [x] `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` pass
- [x] CI workflow runs in a container and is green on a pushed branch

### Phase 2: Config and matching

- [x] T3: Config model, TOML parsing, validation
- [x] T4: Pattern and template engine

**Checkpoint: Config**
- [x] Invalid TOML, missing fields, bad queue capacity, and empty sources fail with precise errors
- [x] Named captures, `{line}` substitution, and bad regex are tested

### Phase 3: Tail engine

- [x] T5: Tail core: EOF start, complete lines, partial buffer, line cap
- [x] T6: inotify watch and event loop
- [x] T7: Rotation and inode handling

**Checkpoint: Tail**
- [x] Append emits exactly one event per complete line
- [x] Partial line is buffered
- [x] 64 KiB line is capped to one event without panic
- [x] copytruncate resets and reads new content
- [x] inode replacement reopens at end

### Phase 4: Cooldown, queue, notifications

- [x] T8: Per-source cooldown
- [x] T9: Bounded queue, notifier worker, zbus backend, dropped log

**Checkpoint: Notifications**
- [ ] Second match inside the window is suppressed
- [ ] Match after the window is enqueued
- [ ] Queue overflow drops the oldest event of that source and logs it
- [ ] Transient D-Bus failure retries, then logs a drop
- [ ] Missing daemon logs once per dropped event and tailing continues
- [ ] `--dry-run` never touches D-Bus

### Phase 5: Daemon wiring and proof

- [x] T10: CLI, config path resolution, lifecycle
- [x] T11: Lockfile and signal handling
- [x] T12: Fixture integration tests
- [x] T13: Default config and config-only behavior
- [x] T14: Release CI and musl artifact

**Checkpoint: v0.1 complete**
- [ ] All spec §7 tests pass
- [ ] Manual `--dry-run` against fixture logs produces the expected events
- [ ] README commands work from a clean checkout
- [ ] Release workflow produces a binary artifact
- [ ] Default config loads; adding or removing a source needs only a TOML edit

## Parallelization

- After T1: T3 and T5 can run in parallel; T8 can also start.
- T4 depends on T3; T6 depends on T5; T7 depends on T6.
- T9 depends on T4 and T8.
- T10 depends on T3, T7, and T9.
- T12 and T13 depend on T10 and T9.
- T14 depends on T12.

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Queue overflow under burst or slow daemon | Medium | Bounded per-source queue, drop oldest, log every drop |
| Restart loses pending events | Medium | Documented; no persistence by design |
| Retry duplicates a notification | Low | Accept duplicate over silent loss |
| Dropped log not writable | Medium | One warning, stderr fallback |
| Missing or unreadable sources | High | Warn and watch nearest ancestor for missing; fail loud on permissions |
| inotify edge cases on rename and delete | High | Test copytruncate, rename, delete/recreate, inode replacement |
| Non-UTF-8 log content | Medium | Lossy conversion; fixture test |
| zbus 6.5 API changes | Low | Pin minor version; async API + `block_on` is the boundary |
| Notification call blocks worker | Medium | 5 s method timeout, bounded retries, bounded queue |
| CI policy conflict | Medium | Run the GitHub Actions job inside a Rust container |
| Default config paths absent on dev hosts | Medium | Use fixture paths for tests; never point tests at system logs |

## Resolved Decisions

All v0.1 behavior questions are closed:

1. **Missing source at startup.** Warn, watch the nearest existing ancestor
   directory, continue. Re-resolve when intermediate directories are created.
   Permission denied is fatal.
2. **Non-UTF-8 lines.** `String::from_utf8_lossy` per line; no panic.
3. **`$XDG_RUNTIME_DIR` unset.** Use `/run/user/<uid>` if present, else exit
   with a clear error.
4. **File event crate.** Direct `inotify` 0.11.x.
5. **Rust edition and MSRV.** Edition 2024, stable pinned via
   `rust-toolchain.toml`, no MSRV promise for v0.1.
6. **`--once`.** Exit after the first regex match is processed. Normal mode
   waits for the worker's delivery or drop outcome before exiting.
7. **Default config paths.** `/etc/log-notify/log-notify.toml`, then
   `~/.config/log-notify/log-notify.toml`.
8. **`replaces_id`.** `0` in v0.1; per-source reuse in v0.2.
9. **Notification guarantee.** In-memory bounded per-source queue, default 10,
   maximum 100. No persistence. Dropped events go to
   `/var/log/log-notify.log` by default, with stderr fallback.
10. **Retry policy.** 3 retries, 1/2/4 s backoff, 5 s method timeout. Missing
    service or exhausted retries: drop and log.
11. **Default config coupling.** None. Patterns, sources, templates, urgency,
    cooldown, and queue capacity are TOML data. Future log-format changes are
    config-only edits.
12. **Repository remote.** Public `skea999/log-notify`; first push done.

## Spec Deltas

These refine the source spec and are recorded in ADR-0008:

- The source spec says a missing notification daemon drops the event. v0.1
  retries transient failures first, then drops and writes the event to the
  dropped-event log.
- The source spec logs to stderr only. v0.1 adds a configurable dropped-event
  log for undeliverable and overflow events.
- The source spec has no queue. v0.1 adds a bounded in-memory per-source queue,
  default 10, maximum 100. There is still no persistence or replay.
