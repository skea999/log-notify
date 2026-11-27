# Project Context

`log-notify` — single-binary Linux daemon that tails configured log files,
matches per-source regexes, and emits freedesktop desktop notifications through
D-Bus. It is a notifier, not a security control.

**Stack:** Rust stable (edition 2024), Linux only. inotify-driven, no polling.
D-Bus `org.freedesktop.Notifications` via zbus 6.5. TOML config, Rust `regex`
patterns. Main loop never calls D-Bus; a bounded queue and a dedicated notifier
worker isolate delivery.

**State:** v0.1 implementation complete (2026-11-27); first release tag pending.
Canonical work list: `tasks/todo.md`.

## Project Structure

```text
src/                 Rust implementation
src/main.rs          CLI, startup, config load, fail-loud validation
src/config.rs        TOML parse and validation, regex compilation
src/tail.rs          inotify tailer: offsets, partial lines, shrink reset
src/notify.rs        notification queue, worker, dropped log, zbus backend
src/cooldown.rs      per-source rate limiting
tests/               smoke, pattern, tail, cooldown, config tests
config/              shipped default config
ci/                  CI container image
scripts/             test, static, docs, duplicate, release gates
flake.nix            Nix package from the pinned release tarball
nix/binary.nix       version/url/sha256 pinned on tag
docs/                spec, architecture, ADRs, project and task docs
docs/decisions/      Architecture Decision Records
tasks/               implementation plan and ordered task list
state/               session state
```

The repository was created from an OpenCode template. Files used by the agent
harness (`package.json`, `.opencode/`, `.agents/`, `.githooks/`, `scripts/`,
`references/`) stay. Project code goes only under `src/`.

## Commands

Containerized gates (host toolchain is not used):

```bash
scripts/dev.sh image      # build log-notify-ci:ci
scripts/dev.sh check      # fmt + clippy-driven test gate
scripts/dev.sh static     # full static analysis
scripts/dev.sh docs       # markdownlint-cli2
scripts/dev.sh verify     # static + docs + static-musl release package
nix build                 # flake package from the pinned musl tarball
```

Direct commands for a host with Rust:

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
- Every notification failure is logged truthfully; never retry-spam.
- Main loop never blocks on D-Bus. Per-source queues are bounded (`10` default,
  `100` max); overflow and terminal delivery failures go to the dropped-event log.
- Unit tests cover config, pattern substitution, tail/rotation, cooldown, queue
  overflow, retry, and dropped-event logging.
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
- `tasks/plan.md` — implementation plan and resolved decisions
- `docs/global-setup.md`, `docs/update.md`, `docs/repo-setup.md` — harness docs

## Boundaries

- **Always:** read logs only; start at end of file; compile and validate all
  regexes at startup; keep the daemon useful without a notification service;
  keep the main loop free of D-Bus calls; update plan/state when a decision
  changes.
- **Ask first:** add or replace dependencies; change the notification backend;
  change queue capacity or dropped-log defaults; change shipped default
  patterns; change config schema; change CI or release configuration.
- **Never:** add native tool hooks; add notification replay or persistence;
  write to monitored logs; guess log patterns without real output; commit
  secrets; make config errors silent; let a D-Bus call block tailing.
