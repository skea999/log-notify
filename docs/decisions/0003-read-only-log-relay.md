# ADR-0003: Read-only log relay, no native hooks

## Status

Accepted — 2026-11-26. Deciders: project owner, implementation agent.

## Context

Security tools expose events in different ways. Some offer native hooks (for
example clamd `VirusEvent`), some write logs, and some do both. Per-tool hook
integration is non-uniform and couples the notifier to each tool's configuration.
Users also do not want old events replayed after login.

## Decision

We will read log files only. `log-notify` does not install native hooks, does not
write to monitored logs, and does not persist event state. Every source starts at
end of file. The logs themselves are the record. Notification replay, unread
counters, and cursor files are non-goals.

## Options

- **Read-only log relay** — Selected. One uniform input path; no per-tool
  integration; least privilege.
- **Native tool hooks** — Rejected. Non-uniform, requires editing every tool's
  config, and breaks when a tool changes.
- **Journald or syslog input** — Rejected for v0.1. Not all target tools log
  there, and the spec limits input to files.
- **Persist offsets and replay unread events** — Rejected. Creates duplicate
  notifications and state to recover; explicitly a non-goal.

## Consequences

- A tool must write a log file that the user can read.
- Patterns depend on log formats and can drift; they need verification against
  real output.
- No event survives a restart. This is intentional.
- Rotation handling is our responsibility: shrink reset and inode reopen.
