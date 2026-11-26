# Tasks — log-notify

**Milestone**: v0.1.0
**Updated**: 2026-11-26

Canonical task list: [`tasks/todo.md`](../tasks/todo.md).
Implementation plan: [`tasks/plan.md`](../tasks/plan.md).

## Current Sprint

### Todo

- [ ] T1 — Cargo scaffold, toolchain, module skeleton
- [ ] T2 — Containerized CI gates
- [ ] T3 — Config model, TOML parsing, validation
- [ ] T4 — Pattern and template engine
- [ ] T5 — Tail engine: EOF start, complete lines, line cap
- [ ] T6 — inotify watch and event loop
- [ ] T7 — Rotation and inode handling
- [ ] T8 — Per-source cooldown
- [ ] T9 — Bounded queue, notifier worker, zbus backend, dropped log
- [ ] T10 — CLI, config path resolution, lifecycle
- [ ] T11 — Lockfile and signal handling
- [ ] T12 — Fixture integration tests
- [ ] T13 — Default config and config-only behavior
- [ ] T14 — Release CI and musl artifact

### In Progress

None.

### Done

- [x] Project initialization: docs, ADRs, plan, task list, state — 2026-11-26

## Backlog

- SIGHUP config reload (v0.2)
- Per-source `replaces_id` reuse (v0.2)
- Packaging and user service integration (downstream, FeralOS repositories)

## Known Issues

| Issue | Status | Workaround |
|-------|--------|------------|
| Session-memory scripts fail on this host with Nix libc relocation errors | Open | Use context-mode `ctx_*` tools and `state/session-state.json` |
| CI workflow does not exist yet | Open | Create it in T2, then run on the pushed branch |
| `/var/log/log-notify.log` may not be writable by a user service | Open | Daemon warns once and falls back to stderr; service unit can grant access or set `general.dropped_log` |

---

*Update at session start and end. Full task detail lives in `tasks/todo.md`.*
