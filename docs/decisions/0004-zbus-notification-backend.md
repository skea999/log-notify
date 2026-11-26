# ADR-0004: zbus 6.5 notification backend

## Status

Accepted — 2026-11-26. Deciders: project owner, implementation agent.

## Context

Notifications must reach any freedesktop notification daemon on Wayland or X11.
The source spec recommends zbus, a pure-Rust D-Bus client with no libdbus C
dependency. The current stable release is zbus 6.5. zbus 6 removed the 5.x
blocking API, but provides `zbus::block_on` to drive the async API from
synchronous code.

Reliability is the priority. A hung or slow D-Bus call must not stop log tailing.
A dedicated notifier worker isolates the D-Bus call from the inotify loop.

## Decision

We will use zbus 6.5.x with its async API inside a dedicated notifier worker
thread. The worker drives each notification future with `zbus::block_on`; there
is no async runtime in the daemon.

- Call `org.freedesktop.Notifications.Notify` with `app_name = "log-notify"`.
- `replaces_id = 0` in v0.1; per-source reuse is v0.2 polish.
- `urgency` hint: `low = 0`, `normal = 1`, `critical = 2`.
- `expire_timeout`: `-1` for low/normal; `0` for critical (never expire).
- Set `ConnectionBuilder::method_timeout` to 5 seconds so a missing reply cannot
  hang the worker forever.
- The notifier trait keeps D-Bus out of tests; `--dry-run` never touches D-Bus.

Queueing, retry policy, and dropped-event logging are ADR-0008.

## Options

- **zbus 6.5 async API + `block_on` in a worker** — Selected. Current release,
  no async runtime, D-Bus isolated from tailing.
- **zbus 5.x blocking API** — Rejected. Older major and known upgrade debt; its
  blocking API is gone in 6.x.
- **Full async daemon** — Rejected. Adds tokio or async-io, async inotify, async
  shutdown, and async tests without a notification-latency gain.
- **`notify-send` subprocess** — Rejected. One process per event and less control
  over hints, expiry, and failure handling.
- **libnotify through FFI** — Rejected. Adds a C dependency and unsafe boundary.

## Consequences

- No async runtime is added; the worker blocks only itself on zbus.
- A slow or hung notification daemon cannot stop log tailing. The queue bounds
  memory and overflow is logged.
- `zbus::block_on` is a bridge, not the removed blocking API. The zbus async API
  remains the integration point.
- The worker may retry a call whose reply was lost; a duplicate notification is
  possible. For a security notifier, duplicate is preferable to silent loss.
- zbus 6.5 is the latest stable major. Upgrades stay within the async API.
