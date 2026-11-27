# Changelog

All notable changes to this project are documented in this file (Keep a Changelog format, newest first). Policy: `.opencode/rules/workflow.md`.

## [0.1.0] — 2026-11-27

### Added

- Project initialization for `log-notify`: README with CLI and config reference, project context, domain vocabulary, architecture, roadmap, project brief, implementation plan, and ordered task list.
- ADRs 0001–0008: ADR process, Rust daemon, read-only log relay, zbus 6.5 backend, TOML fail-loud config, ISC license, SIGHUP reload deferred to v0.2, and bounded notification queue.
- `state/session-state.json` now tracks log-notify goals, decisions, risks, and next actions.
- Containerized CI adapted from Ursine: one verify job, Alpine CI image, test gate, static analysis, docs lint, duplicate gate, SBOM, and tag-driven static-musl release package.
- Rust scaffold: `Cargo.toml`, `rust-toolchain.toml`, module skeletons, `--version` smoke test, and the shipped default config.
- T3 config layer: serde/toml schema with `Spanned` errors, documented defaults, queue capacity range, source validation, default path resolution, and 14 tests.
- T4 pattern engine: compile regexes once at load, `{line}` and named captures, `{{`/`}}` literals, and config-load errors with precise spans; 12 tests.
- T5 tail core: EOF start, partial-line buffering, capped long lines with truncation flag, lossy UTF-8, file identity/offset tracking, and 10 tests.
- T6/T7 watcher: inotify ancestor-directory watches, poll plus shutdown pipe, missing/intermediate sources, copytruncate reset, inode reopen, delete/recreate, multi-source indices, and 8 tests.
- T8 cooldown: per-source monotonic windows with injected `Instant`, inclusive boundary, zero disables, and 6 tests.
- T9 delivery: bounded per-source queue, worker thread, dropped-event log, zbus 6.5 `block_on`, 5 s method timeout, 3 retries at 1/2/4 s, dry-run notifier, and 6 tests.
- T10 CLI: clap flags (`--config`, `--dry-run`, `--once`, `--verbose`, `--version`), active-source mapping, dry-run/once lifecycle, watcher warnings, worker startup/shutdown, and 3 CLI tests.
- T11 lifecycle: `flock` single-instance lock with XDG fallback, SIGTERM/SIGINT shutdown through the watcher pipe, SIGHUP ignored, second-instance and SIGTERM CLI tests.
- T12/T13 fixtures: representative fail2ban, clamav, tracee, aide, and auditd lines; shipped config loads and every source pattern matches. No tool-specific logic in Rust.
- T14 release workflow: containerized tag build, static musl tarball with sha256, GitHub release upload, and quality gates before packaging. Local package verification passes; tag publish pending approval.
- T14 Nix flake: pinned static-musl package, PR/main lint + eval job, tag-time `nix/binary.nix` pin, and flake source attachment to the release.
- Local container entry point `scripts/dev.sh` with the same gates as CI.

### Changed

- Replaced skea-store template content in `AGENTS.md`, `CONTEXT.md`, `ROADMAP.md`, `docs/PROJECT.md`, and `docs/TASKS.md`.
- Resolved v0.1 behavior decisions: missing-source warning and ancestor watch, lossy UTF-8, XDG fallback, direct inotify 0.11.x, edition 2024, first-match `--once`, config paths, `replaces_id` deferral, and generic config-only sources.
- Switched notification delivery to zbus 6.5 with a dedicated worker, bounded per-source queue (default 10, max 100), 3 retries with 1/2/4 s backoff, 5 s method timeout, and dropped-event log at `/var/log/log-notify.log` with stderr fallback.
- Recorded the initial documentation-only scope; the Rust scaffold and CI are now in place.

### Notes

- License decision: ISC, copyright skea999 (user choice 2026-11-26).
- Default config is validated data; changing sources or patterns needs only a TOML edit.
- Public remote created: `https://github.com/skea999/log-notify`; first documentation commit pushed.
- GitHub Actions `verify` run `45864736126` is green on `main`; the release job is tag-gated.
- Template harness files (`package.json`, `.opencode/`, `.agents/`, `scripts/`, `references/`) are intentionally kept; project code goes under `src/`.

## [Unreleased] — 2026-11-25

### Added

- `start-project` primary agent: new-project documentation bootstrap chain (`interview-me` → `spec-driven-development` → `planning-and-task-breakdown` → `project-bootstrap` → `adr` → `api-documentation` → `grill-with-docs`); registered in `docs/agents-guide.md` and `.opencode/rules/start-project.md`.

### Changed

- Trimmed always-loaded `AGENTS.md` to artifacts, docs pointers, and deferred decisions; moved workflow/direct/changelog into `.opencode/rules/workflow.md`, agent/skill provenance into `docs/agents-guide.md` and `docs/update.md`, and git-hook setup into `docs/repo-setup.md`.
- Moved session-memory commands out of always-loaded `AGENTS.md` into conditionally injected `.opencode/rules/session.md` (keywords include `commit`, `push`, `tag`).
- Moved the new-project start workflow out of always-loaded `AGENTS.md` into conditionally injected `.opencode/rules/start-project.md`.
- Moved container execution policy and docker/compose/helm hardening out of always-loaded `AGENTS.md` and `coding-standards` into `.opencode/rules/containers.md`, injected only on container/CI/test context.
- Removed the per-agent `steps` cap from every `.opencode/agents/*.md` frontmatter; agents now run without an explicit model-step limit.

### Fixed

- Pre-commit hook passed patterns starting with `-` as grep options; added `--` so the private-key regex actually runs instead of silently erroring.
- agents-opencode plugin failed to load on OpenCode v2.3.0 (`PluginModule.LoadError: Plugin must export a default definition`): `.opencode/` pack regenerated 2.6.0 → 4.3.5 with `--target v2`. Plugin now uses `export default`; installer removed the retired `agents-opencode` entry from the `opencode.json` plugin array.
- Agent split: pack `researcher` is installer-managed again (manifest entry restored) and the custom Context7 agent is preserved as `.opencode/agents/custom-researcher.md` (V2 frontmatter, agent map + orchestrator allowlist, kept out of the manifest).
- Manifest `rootDir` corrected from the stale `skea-store` path to this repository.
- `ctx_doctor` now passes every check: context-mode `2.0.0-next.40` → `2.2.3`, plugin hooks + `mcp.context-mode` tools configured by official `context-mode setup`, global config renamed `opencode.jsonc` → `opencode.json` (doctor reads only that filename).

### Changed

- Harness refreshed per `docs/update.md`: caveman + cavecrew reinstalled globally; generated duplicate V1 `plugin` key merged into the V2 `plugins` array; cavecrew agents re-downloaded.
- Universal skills updated with `npx skills update` (24 refreshed). Upstream-deleted `improve-codebase-architecture` kept locally.
- Template agent packs `quality` + `security` reinstalled (10 agents). Nested `devtools/`/`security/` outputs promoted to flat `.opencode/agents/` and nested dirs removed, keeping one copy per agent and matching the orchestrator allowlist.
- OpenCode CLI stays 2.3.0: `nix profile upgrade opencode` reports no newer nixpkgs build (npm has 2.4.1, but the host installs via nix).
- `AGENTS.md` / `docs/agents-guide.md`: Tab-selectable agent and user-created list now name `custom-researcher`; pack `researcher` documented as managed subagent.
- `docs/global-setup.md`: context-mode section rewritten for 2.2.3 (plugin hooks + MCP tools, `opencode.json` filename, official setup CLI); version snapshot updated.

## [Unreleased] — 2026-10-16

### Changed

- opencode: removed static `model` overrides from `code-reviewer`, `information-scan`, `vulnerability-scan` in `opencode.json` — all agents and subagents now use the model selected in opencode (no agent md file carried a model override). Verified in merged config: zero agents with model override.
- opencode permissions (global + project): everything allowed except privilege escalation — `sudo`/`sudo *`/`doas`/`doas *` bash patterns denied (doas included: it is Alpine's sudo-equivalent). Verified in merged config.
- ast-grep installed globally on the host (`apk`, Alpine pkg v0.28.1 — npm/GitHub ship gnu-only builds, no musl; project-local npm install impossible on this host). Agent skill installed globally (`~/.agents/skills/`: `ast-grep` + `ast-grep-outline` — opencode loads that dir natively). Preference rules in global `AGENTS.md` (`ast-grep-begin` block). Documented in `docs/global-setup.md` §4.
- `context-mode` v1.0.169 installed as a GLOBAL plugin (`~/.config/opencode/opencode.jsonc` plugin array; never `mcp.context-mode` — both together register zero `ctx_*` tools). Routing note added to global `AGENTS.md` (`context-mode-begin` block). Verified in merged config; live check = `ctx stats` in a session.
- `compaction: {auto: true}` restored in global config (accidentally dropped during the context-mode config rewrite).
- `docs/global-setup.md` rewritten: version snapshot (opencode 1.15.10, agents-opencode 2.6.0, skills split project/global, context-mode 1.0.169, ast-grep 0.28.1), context-mode + ast-grep sections, stale caveman patching caveats removed (upstream fixed), memory-plugin evaluation notes (agentmemory rejected — V2/Windows-only; mathew-cf rejected — global store; opencode-memory-pro parked as per-project candidate).
- Rules/workflows synced with v2.6.0 agents and updated skills: `rules/workflow.md` skill-chain now includes `decision-grounding` (planning), `code-change-impact` (pre-implementation blast radius), `security-audit` (security lens), `adr` (architecture docs); agent-chain notes `@review` lens dispatch (Pattern 8) and full review parallel set. `AGENTS.md` Direct path lists exactly the Tab-selectable agents and points to `decision-grounding`/`adr` skills.
- `opencode.json`: orchestrator can now delegate to `legacy-modernizer`, `compliance-auditor`, `smart-contract-auditor`.
- `docs/agents-guide.md`: restructured — "drive directly" (Tab: `plan`, `build`, `codebase`, `planner`, `legal-advisor`, `researcher`, `blogger`, `em-advisor`, `orchestrator`) vs "subagents" (dispatched via `task`, not user-callable). Old doc wrongly said all agents were `@name`-invocable.
- Harness updates (per `docs/update.md`): universal skills updated (41 checked via `npx skills update`), agents-opencode package updated to v2.6.0 (47 files), caveman + cavecrew global components reinstalled (22 skills, new `ultracave`/`megacave` commands), template agents reinstalled (quality + security packs).
- Upstream cavecrew agent files no longer need the `tools`-object or `hidden: true` fixes from `docs/global-setup.md` — frontmatter now uses `mode: subagent` and no `tools` array.
- `.opencode/.agents-opencode-manifest.json`: removed `researcher.md` entry — user-created agent, must not be overwritten by `npx agents-opencode --update`.
- Harness policy: container-only execution — CI pipelines, code tests, deduplication, security checks (secrets/PII/SAST/vuln), and dependency checks run inside containers only. The host is never touched.
- Harness policy: all artifacts and temp files (build output, test results, scan reports, logs) go under `artifacts/<tool-or-task>/`, which is fully gitignored. Host `/tmp` is never used.
- Harness policy: container/compose volumes must map into `artifacts/` only; never mount host `/tmp` or paths outside the workspace.
- Harness policy: keep only the last 3 artifact versions per subfolder; prune older runs to avoid filling the host disk.
- `AGENTS.md` no longer receives change logs — `CHANGELOG.md` is the single source for notable changes.

### Fixed

- `researcher` agent restored from git after v2.6.0 update overwrote the custom version (manifest had it listed as managed).
- Template agents pack installed into `.opencode/agents/devtools/` and `.opencode/agents/security/` subdirs, which opencode does not load — the 3 new agents (`legacy-modernizer`, `compliance-auditor`, `smart-contract-auditor`) were invisible. Promoted them to flat `.opencode/agents/`, removed the pack dirs (7 remaining files were byte-identical duplicates of manifest-managed agents).
- `.gitignore`: `!artifacts/.gitkeep` negation did not work (`artifacts/` excluded the whole dir); now `artifacts/*` + `!artifacts/.gitkeep` — verified with `git check-ignore`.
- `.gitignore`: env pattern now `.env*` + `!.env.example` (previously `.env.development`/`.env.production` were not ignored).
- `.githooks/pre-commit`: removed host-side `npx lint-staged` (violated container-only policy); hook keeps only the pure-bash secrets/PII scan.

### Removed

- Stale skill reference files replaced upstream (old checklists/`.gitkeep` files under `.agents/skills/`).

### Docs

- `docs/agents-guide.md`: added the 3 new agents, updated `@review` (lens briefs) and `@em-advisor` (meetings instructions) descriptions.
- `AGENTS.md`: added Changelog Policy, Container-Only Execution, Artifacts and Temp Files, Artifact Retention sections; caveman-style pass on the new policy sections (wording only, rules unchanged).
- `handoff/latest.md`: milestone log now points to `CHANGELOG.md` instead of `AGENTS.md`.

### Notes

- Upstream deleted skill `improve-codebase-architecture` (from `anomalyco/opencode`); local copy kept, deletion skipped in non-interactive mode.
- New untracked marker `.opencode-agents-version` at repo root (written by agents-opencode v2.6.0) — decide: track or gitignore.
- Local git identity for this repo: `skea999 <sket999@disroot.org>` (repo-only config).
