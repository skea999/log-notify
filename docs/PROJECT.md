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
5. Applies per-source cooldown, then enqueues the event in a small bounded
   in-memory queue.
6. A dedicated worker sends freedesktop notifications through zbus 6.5, with
   urgency, template substitution, bounded retries, and a 5 s method timeout.
7. Handles copytruncate and inode replacement.
8. Keeps tailing, with truthful stderr logs, when no notification daemon exists.
   Undeliverable or overflow events go to the dropped-event log.

The shipped default config targets the FeralOS security stack; it is a starting
point, not a hard dependency.

## Metrics

| Metric | Target | How |
|--------|--------|-----|
| Startup validation | Invalid config or regex exits non-zero with file, line, reason | Config tests and manual checks |
| Notification latency | Event within 1 s of a complete line being written | Fixture integration test |
| Tailing isolation | Main loop never calls D-Bus; a hung daemon does not stop reads | Code review and fault-injection test |
| Queue bound | `sum(queue_capacity) * line_cap_bytes`; default 10 events per source, max 100 | Config validation and queue tests |
| Delivery retry | 3 attempts, 1/2/4 s backoff, 5 s method timeout | Worker tests |
| Dropped events | Every drop logged with source and reason | Dropped-log tests |
| History replay | Zero events from pre-start content | Tail test: append after start only |
| Idle CPU | No polling; idle work driven by inotify | Code review and runtime check |
| Memory | Few MB RSS for the default source set | Manual measurement |
| Long lines | 64 KiB line capped, one event, no panic | Tail unit test |
| Rotation | copytruncate and inode replacement both recover | Tail unit tests |
| Notification absence | Daemon keeps tailing; drops are logged | Integration test with failing notifier |

## Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Shipped patterns drift from real log formats | Medium | Missed or noisy notifications | Config is data; fix the TOML only. Default set was validated against target logs. |
| Configured log file missing at startup | Medium | Silent gap if the file appears later | Warn, watch the nearest existing parent, continue. Permission denied is fatal. |
| Non-UTF-8 log lines | Medium | Match failures or panic | Lossy conversion per line; fixture test with invalid bytes. |
| Notification daemon absent or D-Bus drops | Medium | No visible notification | Retry transient errors 3 times, then drop and log. Tailing continues. |
| Queue overflow under a burst or slow daemon | Medium | Oldest notifications dropped | Bounded per-source queue, default 10, max 100; every drop logged with reason. |
| Retry after an uncertain reply | Low | Duplicate notification | Accepted: duplicate is better than silent loss. Document it. |
| Dropped-event log unwritable | Medium | Lost diagnostic record | One stderr warning, fall back to stderr; service unit should grant access. |
| Process restart | Medium | Pending in-memory events lost | Documented; no persistence by design. Logs remain the record. |
| Log rotation edge cases | Medium | Lost or duplicated lines | Shrink reset, inode reopen, fixture tests. |
| Regex matches sensitive log content and displays it | Low | Data exposure on screen or in dropped log | Document that bodies contain log lines; users can change templates; protect the dropped log. |
| CI and release workflow unvalidated | Medium | Broken first release | Remote exists; run CI and a test tag before v0.1.0. |
| zbus 6.5 API changes | Low | Backend adjustment | Pin the minor version; the async API and `block_on` are the integration point. |

## Out of Scope (v1)

- Notification replay, persistence, unread counters, cursor files.
- Durable spool; the queue is in-memory and bounded.
- GUI or configuration editor.
- Log storage, indexing, search, or central collection.
- Filtering beyond per-line regex.
- Multi-user routing.
- Remote, journald, or syslog input. Files only.
- Native tool hooks.
- FeralOS packaging and service units (downstream).
