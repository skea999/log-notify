# ADR-0004: zbus notification backend

## Status

Accepted — 2026-11-26. Deciders: project owner, implementation agent.

## Context

Notifications must reach any freedesktop notification daemon on Wayland or X11.
The source spec recommends zbus, a pure-Rust D-Bus client with no libdbus C
dependency, and allows `notify-send` as a lighter alternative. zbus 6 removed the
blocking API, so the version choice affects the daemon's concurrency model.

## Decision

We will use `zbus` 5.x with its blocking API for v0.1. The call targets
`org.freedesktop.Notifications.Notify` with `app_name = "log-notify"`,
`replaces_id = 0` in v0.1, an urgency hint (`low = 0`, `normal = 1`,
`critical = 2`), and `expire_timeout = -1` for low/normal and `0` for critical.

Notification failures are logged to stderr and the event is dropped. The daemon
keeps tailing. There is no retry loop. A notifier trait isolates D-Bus from
tests; `--dry-run` is a separate implementation that prints to stdout.

## Options

- **zbus 5.x blocking** — Selected. Pure Rust, no C dependency, fits the
  synchronous event loop, full control over hints and expiry.
- **zbus 6.x async** — Rejected for v0.1. Requires an async runtime or a
  blocking bridge; more complexity than the workload needs.
- **`notify-send` subprocess** — Rejected. One process per event, less control
  over urgency hints and expiry, and parsing failures are harder to handle.
- **libnotify through FFI** — Rejected. Adds a C dependency and unsafe boundary.

## Consequences

- The dependency tree is larger than a subprocess call, but the binary remains
  self-contained.
- zbus 5.x must be pinned. Upgrading to 6.x requires an async refactor or a
  backend replacement; track this in the roadmap.
- A local D-Bus call can block briefly; if it becomes a tailing problem, move
  notifications to a worker thread.
- `replaces_id = 0` means bursts can stack windows until v0.2 adds per-source
  reuse. Cooldown limits the burst size.
