# Tasks — log-notify

**Milestone**: v0.1.0 released 2026-11-27
**Updated**: 2026-11-27

Canonical task list: [`tasks/todo.md`](../tasks/todo.md).
Implementation plan: [`tasks/plan.md`](../tasks/plan.md).

## Current Sprint

### Todo

None.

### In Progress

None.

### Done

- [x] T1 — Cargo scaffold, toolchain, module skeleton — 2026-11-26
- [x] T2 — Containerized CI gates — 2026-11-26; GitHub Actions run `45864736126` green
- [x] T3 — Config model, TOML parsing, validation — 2026-11-26; 17 tests
- [x] T4 — Pattern and template engine — 2026-11-26; 12 tests
- [x] T5 — Tail core: EOF start, complete lines, partial buffer, line cap — 2026-11-27; 10 tests
- [x] T6 — inotify watch and event loop — 2026-11-27; 8 watch tests
- [x] T7 — Rotation and inode handling — 2026-11-27; covered by watch tests
- [x] T8 — Per-source cooldown — 2026-11-27; 6 tests
- [x] T9 — Bounded queue, notifier worker, zbus backend, dropped log — 2026-11-27; 6 tests
- [x] T10 — CLI, config path resolution, lifecycle — 2026-11-27; 5 CLI tests
- [x] T11 — Lockfile and signal handling — 2026-11-27; 2 lock tests, SIGTERM and second-instance CLI tests
- [x] T12 — Fixture integration tests — 2026-11-27; 2 integration tests + fixtures
- [x] T13 — Default config and config-only behavior — 2026-11-27; all 5 sources matched
- [x] T14 — Release CI and musl artifact — 2026-11-27; tag `v0.1.0`, binary + flake assets
- [x] Project initialization: docs, ADRs, plan, task list, state — 2026-11-26

## Backlog

- SIGHUP config reload (v0.2)
- Per-source `replaces_id` reuse (v0.2)
- Packaging and user service integration (downstream, FeralOS repositories)

## Known Issues

| Issue | Status | Workaround |
|-------|--------|------------|
| Session-memory scripts fail on this host with Nix libc relocation errors | Open | Use context-mode `ctx_*` tools and `state/session-state.json` |
| `/var/log/log-notify.log` may not be writable by a user service | Open | Daemon warns once and falls back to stderr; service unit can grant access or set `general.dropped_log` |

---

*Update at session start and end. Full task detail lives in `tasks/todo.md`.*
