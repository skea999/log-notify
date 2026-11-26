# ADR-0002: Rust single-binary daemon

## Status

Accepted — 2026-11-26. Deciders: project owner, implementation agent.

## Context

The daemon must tail log files continuously with low overhead, handle untrusted
log content, and ship as a small binary. It runs in a user session on Linux and
must be easy to package. The source spec fixes Rust and a single-binary design.

## Decision

We will implement `log-notify` in Rust stable as one binary. The v0.1 event loop
is synchronous and single-threaded: blocking inotify reads, no async runtime.
The Linux target is the only supported platform. CI pins the toolchain with
`rust-toolchain.toml`.

## Options

- **Rust, single-threaded, no async runtime** — Selected. Memory safety, small
  runtime, simple control flow, easy musl static build.
- **Rust with Tokio** — Rejected for v0.1. Adds a runtime and async complexity
  for a workload driven by one blocking descriptor.
- **Go** — Rejected. Good concurrency and deployment, but larger runtime and
  memory footprint; the spec chose Rust.
- **C** — Rejected. Smallest footprint, but memory-safety risk on parsing
  untrusted lines is not acceptable for a security-adjacent tool.
- **Python** — Rejected. Runtime dependency and higher idle memory; packaging is
  heavier.

## Consequences

- Memory safety and a small binary are available without a GC.
- All I/O is blocking; a slow notification call can delay tailing. The D-Bus
  call is local and bounded; if this becomes a problem, move notifications to a
  worker thread.
- The zbus blocking API pinned in ADR-0004 fits this model. A future zbus 6
  upgrade forces an async refactor or a different backend.
- Cross-platform support is explicitly out of scope; inotify semantics are part
  of the design.
