# ADR-0008: Bounded in-memory notification queue

## Status

Accepted — 2026-11-26. Deciders: project owner, implementation agent.

## Context

The source spec says no notification persistence and no replay after login. The
project owner also requires that notifications are not lost during normal
operation and that the daemon uses very little RAM. A notification burst, a slow
notification service, or a transient D-Bus failure must not stall log tailing or
grow memory without bound.

The source spec says a missing notification daemon should drop the event. That
remains true for a permanently absent service, but dropped events must now be
written to a log file for diagnosis.

## Decision

We will use a dedicated notifier worker and a bounded in-memory queue.

- The main thread reads logs, matches, applies cooldown, and enqueues events. It
  never calls D-Bus.
- The queue is a shared `Mutex<VecDeque<Event>>` with a `Condvar`. The worker
  pops the oldest event and sends it.
- Each source has `queue_capacity`, default `10`, allowed range `1..=100`.
  Memory is bounded by `sum(queue_capacity) * line_cap_bytes`.
- On overflow, drop the oldest event for that source, write the dropped event to
  the dropped-event log, then enqueue the new event. Keep the most recent events.
- `general.dropped_log` defaults to `/var/log/log-notify.log`. If the file
  cannot be opened, log one warning to stderr and fall back to stderr for
  dropped events. The daemon keeps running.
- Dropped-event lines are one line each: timestamp, source path, reason
  (`queue_full`, `notify_failed`, `no_service`), summary, and body. Newlines in
  the body are escaped.
- Retry transient D-Bus failures up to 3 times with 1 s, 2 s, and 4 s backoff.
  backoff. Each call has a 5 s method timeout. On a missing service or after the
  retries are exhausted, drop the event and log it.
- There is no persistence across restart. Pending events are lost on exit. This
  preserves the spec's no-replay rule.

## Options

- **Bounded in-memory queue + worker** — Selected. Low RAM, tailing never blocks
  on D-Bus, transient failures are retried.
- **Unbounded queue** — Rejected. A missing or hung notification service grows
  memory without limit.
- **Durable spool on disk** — Rejected for v0.1. Strongest delivery guarantee,
  but it conflicts with the spec's no-persistence rule, adds replay/duplicate
  semantics, and needs recovery tests.
- **Drop immediately on first failure** — Rejected. Loses notifications on a
  transient D-Bus hiccup.
- **One worker per source** — Rejected. More threads and more memory for no
  benefit at this scale.

## Consequences

- Tailing continues when the notification service is slow, hung, or absent.
- Memory is predictable and small. The default config uses 10 events per source;
  the line cap bounds each event.
- A permanently absent service causes drops, but every drop is logged.
- A restart loses pending events. This is intentional and documented.
- `/var/log/log-notify.log` may not be writable by a user-session service. The
  stderr fallback keeps the daemon running; the service unit should grant write
  access or set a writable path.
- Retrying after an uncertain reply can duplicate a notification. This is
  accepted: duplicate is better than silent loss.
