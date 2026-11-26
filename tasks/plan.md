# Implementation Plan: log-notify v0.1

Source spec: [`docs/log-notify-spec.md`](../docs/log-notify-spec.md)
Architecture: [`docs/architecture.md`](../docs/architecture.md)
Task list: [`todo.md`](todo.md)

## Overview

Build a single-binary Rust daemon that tails configured log files with inotify,
matches one regex per source, applies a per-source cooldown, and sends
freedesktop notifications through D-Bus. Start every source at end of file, never
replay history, survive rotation, and keep tailing when no notification service
exists. v0.1 ends with tested code, a shipped default config, containerized CI,
and a release build.

Documentation initialization is complete. No Rust code exists yet.

## Architecture Decisions

- [ADR-0002](../docs/decisions/0002-rust-single-binary-daemon.md): Rust stable,
  single binary, synchronous single-threaded event loop, Linux only.
- [ADR-0003](../docs/decisions/0003-read-only-log-relay.md): read logs only, no
  native hooks, no replay or persistence.
- [ADR-0004](../docs/decisions/0004-zbus-notification-backend.md): zbus 5.x
  blocking API; urgency mapping; drop and log on failure.
- [ADR-0005](../docs/decisions/0005-toml-config-fail-loud.md): TOML with serde;
  validate schema, regexes, and templates at startup; fail loud.
- [ADR-0006](../docs/decisions/0006-keep-isc-license.md): ISC license.
- [ADR-0007](../docs/decisions/0007-defer-sighup-reload.md): SIGHUP reload in
  v0.2; v0.1 requires restart.

Proposed dependency set, to confirm at scaffold time with Context7 and
`cargo tree`:

| Purpose | Crate | Notes |
|---------|-------|-------|
| CLI | `clap` derive | Version, help, flags. |
| Config | `serde`, `toml` | Reject unknown fields; use `toml::Spanned` for line/column in semantic errors. |
| Matching | `regex` | Linear-time engine; compile at startup. |
| File events | `inotify` or `notify` | Direct `inotify` is smaller; `notify` handles watcher edge cases. Decide in T1. |
| D-Bus | `zbus` 5.x, `blocking-api` | Pin major 5; see ADR-0004. |
| Signals | `signal-hook` | SIGTERM, SIGINT; SIGHUP ignored. |
| Lock | `fs2` or `rustix` | Advisory lock on the runtime path. |
| Errors | `anyhow` or `thiserror` | Typed config errors with file/line context. |

No async runtime. No network dependency.

## Task List

### Phase 1: Foundation

- [ ] T1: Cargo scaffold, pinned toolchain, module skeleton
- [ ] T2: Containerized CI gates

**Checkpoint: Foundation**
- [ ] `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` pass
- [ ] CI workflow runs in a container and is green on a pushed branch

### Phase 2: Config and matching

- [ ] T3: Config model, TOML parsing, validation
- [ ] T4: Pattern and template engine

**Checkpoint: Config**
- [ ] Invalid TOML, missing fields, bad regex, and empty sources fail with precise errors
- [ ] Named captures and `{line}` substitution are tested

### Phase 3: Tail engine

- [ ] T5: Tail core: EOF start, complete lines, partial buffer, line cap
- [ ] T6: inotify watch and event loop
- [ ] T7: Rotation and inode handling

**Checkpoint: Tail**
- [ ] Append emits exactly one event per complete line
- [ ] Partial line is buffered
- [ ] 64 KiB line is capped without panic
- [ ] copytruncate resets and reads new content
- [ ] inode replacement reopens at end

### Phase 4: Cooldown and notifications

- [ ] T8: Per-source cooldown
- [ ] T9: Notifier trait, dry-run, zbus backend

**Checkpoint: Notifications**
- [ ] Second match inside the window is suppressed
- [ ] Match after the window is emitted
- [ ] Missing daemon logs once and tailing continues
- [ ] `--dry-run` never touches D-Bus

### Phase 5: Daemon wiring and proof

- [ ] T10: CLI, config path resolution, lifecycle
- [ ] T11: Lockfile and signal handling
- [ ] T12: Fixture integration tests
- [ ] T13: Default config and pattern verification
- [ ] T14: Release CI and musl artifact

**Checkpoint: v0.1 complete**
- [ ] All spec §7 tests pass
- [ ] Manual `--dry-run` against fixture logs produces the expected events
- [ ] README commands work from a clean checkout
- [ ] Release workflow produces a binary artifact
- [ ] Tracee pattern verified against real output or explicitly flagged as unresolved

## Parallelization

- After T1: T3 and T5 can run in parallel; T8 can also start.
- T4 depends on T3; T6 depends on T5; T7 depends on T6.
- T9 depends on T4 (templates) and T1.
- T10 depends on T3, T7, and T9.
- T12 and T13 depend on T10 and T9.
- T14 depends on T12.

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Tracee pattern is a placeholder | High | Treat as release blocker; collect real tracee JSON and verify before release |
| Missing or unreadable sources | High | Decide policy in T3/T5; fail loud on permissions, warn and watch on absence |
| inotify edge cases on rename and delete | High | Test copytruncate, rename, delete/recreate, and inode replacement explicitly |
| Non-UTF-8 log content | Medium | Lossy conversion; fixture test with invalid bytes |
| zbus 5 blocking API availability | Medium | Confirm at T1; if unavailable, choose zbus 6 with a minimal async bridge or `notify-send` and update ADR-0004 |
| Notification call blocks tailing | Low | Local call; move to a worker thread only if measured |
| CI policy conflict | Medium | Run the GitHub Actions job inside a Rust container to satisfy the container-only rule |
| No remote repository | High | Create the remote before T2 so CI can be validated |
| Default config paths absent on dev hosts | Medium | Use fixture paths for tests; never point tests at system logs |

## Open Questions

1. **Missing source at startup.** Proposed: log a warning, watch for creation,
   and continue. Unreadable file due to permissions: exit non-zero. Confirm.
2. **Non-UTF-8 lines.** Proposed: `String::from_utf8_lossy` per line before
   regex matching. Confirm replacement-character behavior is acceptable.
3. **`$XDG_RUNTIME_DIR` unset.** Proposed: fall back to `/run/user/<uid>` when
   it exists, else exit with a clear error. Confirm.
4. **inotify crate choice.** Direct `inotify` for a smaller tree, or `notify`
   for higher-level watcher handling. Decide in T1 and record if sticky.
5. **Rust edition and MSRV.** Proposed: edition 2021 and a pinned stable
   toolchain for broad packaging compatibility. Confirm at T1.
6. **`--once` and cooldown.** Proposed: exit on the first event that would be
   emitted; cooldown starts empty at process start, so this is the first match.
   Confirm.
7. **Tracee pattern owner.** Who provides real tracee JSON for verification?
   Until then, the pattern is a release blocker.
8. **GitHub remote and owner.** Needed before CI validation and release badges.
9. **Default config installation path.** Confirm `/etc/log-notify/log-notify.toml`
   for FeralOS and `~/.config/log-notify/log-notify.toml` for user installs.
10. **`replaces_id` reuse.** Deferred to v0.2 per roadmap; confirm that burst
    stacking is acceptable for v0.1.
