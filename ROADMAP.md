# Roadmap

**Status:** v0.1 implemented; first release tag pending.

## Milestones

| Ver | Target | Status | What |
|-----|--------|--------|------|
| 0.1.0 | TBD | Implemented | Rust daemon: TOML config, fail-loud validation, inotify tailer, regex matching, cooldown, bounded queue, notifier worker, zbus 6.5, dropped-event log, CLI, tests, containerized CI, default config, tag-driven release with musl artifact. |
| 0.2.0 | TBD | Planned | SIGHUP config reload, per-source `replaces_id` reuse, dropped-log reopen on rotation, packaging polish. |
| Downstream | TBD | Out of scope | FeralOS aports package and user dinit unit. Done in the FeralOS repositories, not here. |

## Pipeline

| Feature | Prio | Status | Notes |
|---------|------|--------|-------|
| Config load and validation | P0 | Planned | TOML, regex compile at startup, precise errors. |
| inotify tailer | P0 | Planned | Offset at EOF, complete lines, partial buffer, line cap, shrink and inode handling. |
| Pattern and template engine | P0 | Planned | `{line}` plus named capture substitution. |
| Per-source cooldown | P0 | Planned | Default 60 s in shipped config; schema default 0. |
| Bounded notification queue | P0 | Planned | Per-source capacity, default 10, max 100; overflow drops oldest and logs. |
| zbus notification backend | P0 | Planned | zbus 6.5, `block_on` in a worker, urgency hint, bounded retries, dropped-event log. |
| CLI and daemon lifecycle | P0 | Planned | `--config`, `--dry-run`, `--once`, `--verbose`, `--version`, lockfile, signals. |
| Test suite | P0 | Planned | Unit and fixture integration tests from spec §7. |
| CI and release build | P1 | Planned | fmt, clippy, test, release build; tag-driven musl artifact. |
| SIGHUP reload | P2 | Deferred | v0.2; v0.1 requires restart. |
| `replaces_id` reuse | P2 | Deferred | Polish after v0.1 behavior is stable. |
| FeralOS packaging | Downstream | Out of scope | aports package and dinit unit live in FeralOS repositories. |

## Timeline

No dates committed. Order follows `tasks/plan.md`.

---

**Updated**: 2026-11-26
**Next review**: after v0.1 checkpoint 1 (foundation)
