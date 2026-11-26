# Project Context

`log-notify` — single-binary Linux daemon that tails configured log files,
matches per-source regexes, and emits freedesktop desktop notifications through
D-Bus. It is a notifier, not a security control.

**Stack:** Rust stable, Linux only. inotify-driven, no polling. D-Bus
`org.freedesktop.Notifications` via zbus. TOML config, Rust `regex` patterns.

**State:** documentation and plan complete (2026-11-26). No Rust implementation
yet. Canonical work list: `tasks/todo.md`.

## Project Structure

```text
src/                 Rust implementation (planned; do not put app code elsewhere)
src/main.rs          CLI, startup, config load, fail-loud validation
src/config.rs        TOML parse and validation, regex compilation
src/tail.rs          inotify tailer: offsets, partial lines, shrink reset
src/notify.rs        notification backend
src/cooldown.rs      per-source rate limiting
tests/               pattern, tail, cooldown, config tests
config/              shipped default config
docs/                spec, architecture, ADRs, project and task docs
docs/decisions/      Architecture Decision Records
tasks/               implementation plan and ordered task list
state/               session state
```

The repository was created from an OpenCode template. Files used by the agent
harness (`package.json`, `.opencode/`, `.agents/`, `.githooks/`, `scripts/`,
`references/`) stay. Project code goes only under `src/`.

## Commands

Planned v0.1 commands:

```bash
cargo build --release
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

Runtime:

```bash
log-notify [--config PATH] [--dry-run] [--once] [--verbose] [--version]
```

## Quality Requirements

- Config and regex validation fail loud: file, line, reason, non-zero exit.
- Every notification failure is logged truthfully to stderr; never retry-spam.
- Unit tests cover config, pattern substitution, tail/rotation, and cooldown.
- No notification daemon in tests: use a notifier trait or `--dry-run`.
- Run fmt, clippy, and tests before committing.
- Update `CHANGELOG.md` for notable changes; never use this file as a changelog.

## Artifacts and Temp Files

All artifacts and temp files — build output, test results, scan reports, logs —
go under `artifacts/<tool-or-task>/` (for example
`artifacts/tests/2026-11-26T1200/`). One subfolder per tool, agent, or run. Never
write host `/tmp` or outside the workspace. `artifacts/` is gitignored; only
`.gitkeep` is tracked. Keep the last 3 versions per artifact subfolder.

## Docs

- `docs/log-notify-spec.md` — source spec and product contract
- `docs/architecture.md` — components, data flow, failure modes
- `docs/decisions/` — ADRs and rationale
- `docs/PROJECT.md` — vision, users, metrics, risks
- `docs/TASKS.md` — milestone summary; canonical list is `tasks/todo.md`
- `tasks/plan.md` — implementation plan and open questions
- `docs/global-setup.md`, `docs/update.md`, `docs/repo-setup.md` — harness docs

## Boundaries

- **Always:** read logs only; start at end of file; compile and validate all
  regexes at startup; keep the daemon useful without a notification service;
  update plan/state when a decision changes.
- **Ask first:** add or replace dependencies; change the notification backend;
  change shipped default patterns; change config schema; change CI or release
  configuration.
- **Never:** add native tool hooks; add notification replay or persistence;
  write to monitored logs; guess log patterns without real output; commit
  secrets; make config errors silent.
