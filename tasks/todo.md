# v0.1 Task List

Plan: [`plan.md`](plan.md)
Source spec: [`../docs/log-notify-spec.md`](../docs/log-notify-spec.md)

Status: all tasks open. Documentation initialization is done.

## Phase 1: Foundation

### Task 1: Cargo scaffold, toolchain, module skeleton

**Description:** Create the Rust crate, pin the toolchain, and lay out the
modules from the spec. Add a `lib.rs` so unit and integration tests can import
the internals while `main.rs` stays thin. Use Rust edition 2024 and the direct
`inotify` 0.11.x crate.

**Acceptance criteria:**
- [x] `Cargo.toml` defines the `log-notify` binary and library.
- [x] `rust-toolchain.toml` pins a stable channel.
- [x] Rust edition 2024 is set in `Cargo.toml`.
- [x] The `inotify` 0.11.x choice is recorded in the plan and ADRs.
- [x] `src/main.rs`, `src/lib.rs`, `src/config.rs`, `src/tail.rs`,
      `src/notify.rs`, and `src/cooldown.rs` exist with documented public stubs.
- [x] `cargo build` succeeds without warnings.

**Verification:**
- [x] `cargo build`
- [x] `cargo fmt --check`
- [x] `cargo clippy -- -D warnings`

**Dependencies:** None.

**Files likely touched:** `Cargo.toml`, `rust-toolchain.toml`, `src/main.rs`,
`src/lib.rs`, `src/config.rs`, `src/tail.rs`, `src/notify.rs`,
`src/cooldown.rs`, `tasks/plan.md`.

**Estimated scope:** Medium (5-8 files).

### Task 2: Containerized CI gates

**Description:** Add a GitHub Actions workflow that runs fmt, clippy, tests, and a
release build. Run the job inside a Rust container to honor the project's
container-only execution rule. Pin the Rust version through the toolchain file.

**Acceptance criteria:**
- [x] Workflow triggers on push and pull request.
- [x] Job runs in a container, not directly on the runner host.
- [x] Steps run `cargo fmt --check`, `cargo clippy -- -D warnings`,
      `cargo test`, and `cargo build --release`.
- [x] Failures block the job; no `continue-on-error`.
- [x] Artifacts and caches, if any, stay inside the workspace or CI cache.

**Verification:**
- [x] Push a branch and confirm the workflow is green.
- [x] Introduce a formatting error locally and confirm the check fails.

**Dependencies:** Task 1.

**Files likely touched:** `.github/workflows/ci.yml`.

**Estimated scope:** Small (1 file).

**Checkpoint: Foundation**
- [x] All tests pass, clippy is clean, and CI is green.
- [x] Review with the human before config work.

## Phase 2: Config and matching

### Task 3: Config model, TOML parsing, validation

**Description:** Implement the TOML schema with `serde`, reject unknown fields,
validate required values, and produce precise errors with file, line, and reason.
Resolve config paths in the documented order. Empty source sets are errors.

**Acceptance criteria:**
- [x] `[general]` supports `line_cap_bytes` with default 8192 and `dropped_log`
      with default `/var/log/log-notify.log`.
- [x] `[[source]]` supports `path`, `pattern`, `summary`, `body`, `urgency`,
      `cooldown_secs`, `queue_capacity`, and `enabled` with the documented
      defaults.
- [x] `queue_capacity` is validated to `1..=100`; default is 10.
- [x] Unknown fields, missing required fields, bad urgency, bad queue capacity,
      and zero sources are rejected with a clear message.
- [x] Semantic errors report the config file, line/column where available, and
      reason; use `toml::Spanned` or equivalent span data.
- [x] Missing config file reports every attempted path.
- [x] Path resolution order is `--config`, `/etc/log-notify/log-notify.toml`,
      `~/.config/log-notify/log-notify.toml`.

**Verification:**
- [x] Unit tests for valid config, unknown field, missing required field, bad
      urgency, bad queue capacity, empty sources, and missing file.
- [x] `cargo test config`

**Dependencies:** Task 1.

**Files likely touched:** `src/config.rs`, `src/lib.rs`, `tests/config.rs`,
`Cargo.toml`.

**Estimated scope:** Medium (3-4 files).

### Task 4: Pattern and template engine

**Description:** Compile each source regex at load time and substitute `{line}`
and named captures into `summary` and `body`. Invalid regex and unknown
placeholder errors must point at the source.

**Acceptance criteria:**
- [x] Regex compilation happens once at startup, not per line.
- [x] `{line}` inserts the full matched line.
- [x] Named capture groups are available as `{name}`.
- [x] Unknown placeholders are rejected at config load.
- [x] Literal braces are handled without panics or silent corruption.
- [x] A line with no match produces no event and no allocation-heavy work.

**Verification:**
- [x] Unit tests for `{line}`, one named capture, multiple captures, unknown
      placeholder, invalid regex, and literal braces.
- [x] `cargo test pattern`

**Dependencies:** Task 3.

**Files likely touched:** `src/config.rs`, `src/lib.rs`, `tests/pattern.rs`.

**Estimated scope:** Medium (3 files).

**Checkpoint: Config**
- [x] All config and pattern tests pass.
- [x] Invalid config never starts the daemon.
- [ ] Review with the human before tail work.

## Phase 3: Tail engine

### Task 5: Tail core: EOF start, complete lines, partial buffer, line cap

**Description:** Implement the per-source state and read path: open at end of
file, read new bytes, buffer the trailing partial line, cap long lines, and emit
complete lines in order. Non-UTF-8 bytes use lossy conversion. An over-cap line
keeps the capped prefix, discards the rest until the newline, and matches once.

**Acceptance criteria:**
- [x] Initial offset equals the current file size; existing content is not read.
- [x] Appended complete lines are emitted exactly once.
- [x] A trailing partial line is buffered until its newline arrives.
- [x] A 64 KiB line is capped at `line_cap_bytes` before matching, with no panic,
      no unbounded memory growth, and at most one event for that line.
- [x] Invalid UTF-8 does not panic; the chosen lossy policy is applied and tested.
- [x] Read errors are reported with the source path and do not kill other
      sources.

**Verification:**
- [x] Unit tests with temp files for append, partial, cap, and invalid bytes.
- [x] `cargo test tail`

**Dependencies:** Task 1.

**Files likely touched:** `src/tail.rs`, `src/lib.rs`, `tests/tail.rs`.

**Estimated scope:** Medium (3 files).

### Task 6: inotify watch and event loop

**Description:** Watch each source with inotify and drive reads from events. Map
watch descriptors to sources, handle read races, watch the nearest existing
ancestor directory for a missing source, and avoid polling.

**Acceptance criteria:**
- [x] Each enabled source has a watch on its path, or on its nearest existing
      ancestor directory when the path is missing.
- [x] `MODIFY`, `CLOSE_WRITE`, and `CREATE` trigger a read or reopen.
- [x] A source created after startup is opened at its current end.
- [x] Intermediate directories created after startup are watched in turn until
      the source path exists.
- [x] Events that do not advance the offset are ignored.
- [x] No busy loop and no timer tick when idle; `poll` blocks with no timeout.
- [x] A failed watch is recorded as a warning with path and reason; other
      sources continue. T10 prints warnings.
- [x] The event loop exits cleanly when the shutdown handle is triggered.

**Verification:**
- [x] Integration test appends to a watched file and observes an event.
- [x] `cargo test watch`

**Dependencies:** Task 5.

**Files likely touched:** `src/tail.rs`, `src/lib.rs`, `tests/tail.rs`,
`Cargo.toml`.

**Estimated scope:** Medium (4 files).

### Task 7: Rotation and inode handling

**Description:** Detect copytruncate and file replacement. On shrink, reset the
offset to 0 and read the new content. On inode change, reopen the path at its
current end. Handle delete and recreate without crashing.

**Acceptance criteria:**
- [x] Shrink detection compares current size with the last offset.
- [x] copytruncate reads the new content from byte 0.
- [x] Inode change closes the old descriptor and reopens the path at EOF.
- [x] Delete then recreate recovers when the path appears again.
- [x] No duplicate events from the old file after a replacement.
- [x] Rotation events are recorded as watcher warnings for T10 `--verbose` output.

**Verification:**
- [x] Unit tests for shrink, inode replacement, delete/recreate.
- [x] `cargo test watch`

**Dependencies:** Task 6.

**Files likely touched:** `src/tail.rs`, `tests/tail.rs`.

**Estimated scope:** Medium (2-3 files).

**Checkpoint: Tail**
- [x] Append, partial, cap, shrink, and inode tests pass.
- [x] A fixture run shows one event per line (tests).
- [ ] Review with the human before notification work.

## Phase 4: Cooldown and notifications

### Task 8: Per-source cooldown

**Description:** Add a per-source monotonic cooldown. After a successful
enqueue, suppress further matches from that source for the configured window.
Use an injectable clock so tests do not sleep.

**Acceptance criteria:**
- [x] Default cooldown is 0 (disabled).
- [x] A second match inside the window is suppressed.
- [x] A match after the window is emitted.
- [x] Cooldown is independent per source.
- [x] Suppressed matches are discarded, not queued.
- [x] Tests use a fake clock; no real sleeps.

**Verification:**
- [x] Unit tests for first match, suppressed second match, expiry, and two
      sources with different windows.
- [x] `cargo test cooldown`

**Dependencies:** Task 1.

**Files likely touched:** `src/cooldown.rs`, `src/lib.rs`, `tests/cooldown.rs`.

**Estimated scope:** Small (3 files).

### Task 9: Bounded queue, notifier worker, zbus backend, dropped log

**Description:** Implement the notification pipeline: a bounded per-source queue,
a dedicated worker thread, the zbus 6.5 backend driven by `zbus::block_on`, a 5 s
method timeout, bounded retries, and the dropped-event log. The main loop must
never call D-Bus. `replaces_id` is 0 in v0.1.

**Acceptance criteria:**
- [x] The queue enforces `queue_capacity` per source (`1..=100`, default 10).
- [x] Overflow drops the oldest event of that source and logs it with reason
      `queue_full`.
- [x] The main loop enqueues and never blocks on D-Bus.
- [x] The worker calls `org.freedesktop.Notifications.Notify` through zbus 6.5
      with the documented arguments and urgency hint.
- [x] `low`/`normal` use `expire_timeout = -1`; `critical` uses `0`.
- [x] The zbus connection uses a 5 second method timeout.
- [x] Transient failures retry 3 times with 1/2/4 s backoff; missing service or
      exhausted retries drop and log with reason `no_service` or `notify_failed`.
- [x] `general.dropped_log` defaults to `/var/log/log-notify.log`; if unwritable,
      warn once and fall back to stderr.
- [x] `DryRunNotifier` prints a stable event line to stdout and never touches
      D-Bus; T10 wires the `--dry-run` flag.
- [x] Unit tests use a fake notifier and fake sleeper; no D-Bus required.

**Verification:**
- [x] `cargo test notify`
- [x] `cargo test queue`
- [ ] `log-notify --dry-run` against a fixture prints the expected event (T10).
- [ ] Manual live test with a running notification daemon (T14).
- [x] Fault test with a failing notifier: drops logged, tailing continues.

**Dependencies:** Task 4, Task 8, Task 1.

**Files likely touched:** `src/notify.rs`, `src/lib.rs`, `tests/notify.rs`,
`tests/queue.rs`, `Cargo.toml`.

**Estimated scope:** Medium (5 files).

**Checkpoint: Notifications**
- [x] Cooldown, queue, worker, and dropped-log tests pass.
- [x] Dry-run and fake notifier paths are deterministic.
- [x] A failing notifier does not stall the main loop.
- [x] Review with the human before wiring the daemon.

## Phase 5: Daemon wiring and proof

### Task 10: CLI, config path resolution, lifecycle

**Description:** Wire the daemon: parse CLI flags, resolve config, validate,
open sources, start the notifier worker, run the event loop, and implement
`--once`, `--verbose`, and `--version`. Ensure startup errors exit non-zero with
precise messages.

**Acceptance criteria:**
- [x] `--config`, `--dry-run`, `--once`, `--verbose`, and `--version` work.
- [x] Unknown flags exit non-zero with usage.
- [x] Config errors print file, line, and reason and exit non-zero.
- [x] `--once` exits after the first regex match; normal mode waits for the
      worker's delivery or drop outcome before exit.
- [x] `--verbose` adds tail-debug lines; normal mode logs one line per event.
- [x] Startup and shutdown leave no orphan watchers or worker threads.
- [x] Startup opens the dropped-event log with stderr fallback.

**Verification:**
- [x] Integration tests for each flag.
- [x] `cargo test cli`

**Dependencies:** Task 3, Task 7, Task 9.

**Files likely touched:** `src/main.rs`, `src/lib.rs`, `tests/cli.rs`.

**Estimated scope:** Medium (3-4 files).

### Task 11: Lockfile and signal handling

**Description:** Add the single-instance advisory lock at
`$XDG_RUNTIME_DIR/log-notify.lock` and handle SIGTERM/SIGINT for graceful exit.
SIGHUP is ignored in v0.1.

**Acceptance criteria:**
- [x] A second instance exits non-zero with a clear message.
- [x] The lock is released on clean exit and on SIGTERM/SIGINT.
- [x] SIGTERM and SIGINT close watchers, wake the worker, and exit 0.
- [x] SIGHUP does not reload; it is logged and ignored.
- [x] `$XDG_RUNTIME_DIR` unset falls back to `/run/user/<uid>` if present, else
      exits with a clear error.

**Verification:**
- [x] Integration test starts two instances; the second fails.
- [x] Send SIGTERM to a running fixture instance; exit code is 0.
- [x] `cargo test lifecycle`

**Dependencies:** Task 10.

**Files likely touched:** `src/main.rs`, `src/lib.rs`, `tests/lifecycle.rs`,
`Cargo.toml`.

**Estimated scope:** Small (3-4 files).

### Task 12: Fixture integration tests

**Description:** Build end-to-end tests with fixture logs: append lines, run the
daemon with `--dry-run`, assert stdout events, exercise cooldown, rotation,
queue overflow, retry/drop logging, and `--once`. Cover the spec §7 minimum bar.

**Acceptance criteria:**
- [x] Pattern engine: named captures, placeholder substitution, invalid regex.
- [x] Tail: append, partial, copytruncate, inode replacement, 64 KiB line.
- [x] Cooldown: suppressed inside window, emitted after.
- [x] Queue: per-source capacity, oldest drop, dropped-event log line.
- [x] Notifier: transient retry, terminal drop, stderr fallback.
- [x] Config: missing file, bad TOML, bad regex, bad queue capacity, empty sources.
- [x] Integration: fixture append produces the correct stdout event.
- [x] Tests do not read real system logs and do not need D-Bus.

**Verification:**
- [x] `cargo test`
- [x] Review the fixture outputs against `docs/log-notify-spec.md` §7.

**Dependencies:** Task 7, Task 9, Task 10.

**Files likely touched:** `tests/fixtures/`, `tests/integration.rs`.

**Estimated scope:** Medium (3-5 files).

### Task 13: Default config and config-only behavior

**Description:** Add `config/log-notify.toml` with the FeralOS security-stack
sources from the spec, plus `dropped_log` and `queue_capacity` defaults. The
supplied pattern set was validated against the target logs. The daemon must stay
generic: sources, patterns, templates, urgency, cooldown, and queue capacity are
data. Adding or removing a source requires only a TOML edit, never a code change.

**Acceptance criteria:**
- [x] Config matches the spec's source set and defaults.
- [x] `line_cap_bytes = 8192`, `dropped_log`, and `queue_capacity = 10` are
      present or documented as defaults.
- [x] Each supplied pattern is tested against a representative fixture line.
- [x] No tool name, log path, or pattern is hardcoded in Rust.
- [x] Adding and removing a source works by editing only the TOML.
- [x] A README command loads the shipped config successfully.

**Verification:**
- [x] Config loads and fixture patterns match in `tests/integration.rs`.
- [x] Pattern checks documented in the task notes or CHANGELOG.

**Dependencies:** Task 4, Task 12.

**Files likely touched:** `config/log-notify.toml`, `tests/fixtures/`,
`README.md`.

**Estimated scope:** Small (2-3 files).

### Task 14: Release CI and musl artifact

**Description:** Add a tag-driven release workflow that builds and uploads the
release binary, including `x86_64-unknown-linux-musl`, plus a Nix flake that
packages the pinned tarball. Keep the same quality gates as CI.

**Acceptance criteria:**
- [ ] Tag push builds the release binary.
- [ ] musl static build succeeds and is uploaded as an artifact.
- [x] fmt, clippy, and tests run before the build.
- [x] The workflow uses a container and pinned toolchain.
- [x] Release notes or changelog entry is produced or documented.
- [x] The flake packages the pinned static tarball and evaluates `0.1.0`.
- [x] The Nix CI job lints/evaluates on PR/main and pins the tag release.

**Verification:**
- [ ] Dry-run the workflow on a test tag.
- [ ] Download the artifact and run `--version`.
- [x] `file` reports a static binary for the musl artifact.

**Dependencies:** Task 12.

**Files likely touched:** `.github/workflows/release.yml`, `CHANGELOG.md`.

**Estimated scope:** Small (1-2 files).

**Checkpoint: v0.1 complete**
- [x] Every spec §7 test passes.
- [x] Manual fixture run matches expected events.
- [ ] README commands work from a clean checkout.
- [ ] Release artifact exists.
- [x] Queue overflow and failed delivery drops are logged.
- [x] Adding or removing a source needs only a TOML edit.
- [ ] Human review and approval before tagging v0.1.0.
