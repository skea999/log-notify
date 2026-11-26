# PROJECT — log-notify

**Status**: Planned (documentation initialized 2026-11-26)
**Stack**: Rust stable, Linux only; inotify + D-Bus

## Vision

One small daemon turns security and system log events into desktop
notifications, without native hooks in every tool and without replaying old
history.

## Users

- Primary: Linux desktop users who run security tooling and want visible events.
- Secondary: FeralOS users and maintainers who need a uniform notification path
  for the default security stack.
- Secondary: packagers and tinkerers who want a small, auditable notifier.
- Not for: teams needing log storage, search, central collection, or multi-user
  routing.

## Problem

Security tools each expose events differently. Some have native hooks, some only
write logs, and wiring every tool separately is fragile. Reading logs uniformly
is simpler, but a naive tailer either replays history, misses rotated files,
spams notifications, or dies when no desktop notification service is present.

## Solution

A single-binary Rust daemon:

1. Reads a TOML config listing log sources and per-source regexes.
2. Starts each source at end of file, so old events are not replayed.
3. Tails with inotify, buffers partial lines, and caps long lines.
4. Applies the source regex to each complete line.
5. Sends a freedesktop notification through D-Bus, with per-source cooldown,
   urgency, and template substitution.
6. Handles copytruncate and inode replacement.
7. Keeps tailing, with truthful stderr logs, when no notification daemon exists.

The shipped default config targets the FeralOS security stack; it is a starting
point, not a hard dependency.

## Metrics

| Metric | Target | How |
|--------|--------|-----|
| Startup validation | Invalid config or regex exits non-zero with file, line, reason | Config tests and manual checks |
| Notification latency | Event within 1 s of a complete line being written | Fixture integration test |
| History replay | Zero events from pre-start content | Tail test: append after start only |
| Idle CPU | No polling; idle work driven by inotify | Code review and runtime check |
| Memory | Few MB RSS for the default source set | Manual measurement |
| Long lines | 64 KiB line capped, no panic | Tail unit test |
| Rotation | copytruncate and inode replacement both recover | Tail unit tests |
| Notification absence | Daemon keeps tailing; one stderr line per failed event | Integration test with failing notifier |

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Shipped patterns drift from real log formats | High | Missed or noisy notifications | Verify each against real output before release; tracee is a known placeholder |
| Tracee JSON pattern is wrong | High | No tracee detections or false positives | Treat as release blocker; validate against real tracee output |
| Configured log file missing at startup | Medium | Daemon exits or silently watches nothing | Decide and test behavior in T1/T2; default proposal: warn, watch for creation, continue |
| Non-UTF-8 log lines | Medium | Match failures or panic | Use lossy conversion per line; add fixture test |
| Notification daemon absent or D-Bus drops | Medium | No visible notification | Drop event, log stderr, keep tailing; never retry-spam |
| Log rotation edge cases | Medium | Lost or duplicated lines | Shrink reset, inode reopen, fixture tests |
| Regex matches sensitive log content and displays it | Low | Data exposure on screen | Document that notification bodies contain log lines; users can change templates |
| No GitHub remote yet | High | CI and release workflow unvalidated | Create remote, push, run CI before release |
| zbus blocking API removed in 6.x | Medium | Upgrade requires async refactor | Pin zbus 5.x for v0.1; record upgrade path in ADR-0004 |

## Out of Scope (v1)

- Notification replay, persistence, unread counters, cursor files.
- GUI or configuration editor.
- Log storage, indexing, search, or central collection.
- Filtering beyond per-line regex.
- Multi-user routing.
- Remote, journald, or syslog input. Files only.
- Native tool hooks.
- FeralOS packaging and service units (downstream).
