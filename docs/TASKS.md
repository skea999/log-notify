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
- [ ] T9 — Notifier trait, dry-run, zbus backend
- [ ] T10 — CLI, config path resolution, lifecycle
- [ ] T11 — Lockfile and signal handling
- [ ] T12 — Fixture integration tests
- [ ] T13 — Default config and pattern verification
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
| Tracee pattern in default config is unverified | Open | Validate against real tracee JSON before release; do not guess |
| Session-memory scripts fail on this host with Nix libc relocation errors | Open | Use context-mode `ctx_*` tools and `state/session-state.json` |
| No GitHub remote or CI workflow yet | Open | Create remote, push, run CI after T2 |

---

*Update at session start and end. Full task detail lives in `tasks/todo.md`.*
