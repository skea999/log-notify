# ADR-0007: Defer SIGHUP config reload to v0.2

## Status

Accepted — 2026-11-26. Deciders: project owner, implementation agent.

## Context

The source spec allows SIGHUP config reload to be deferred to v0.2, provided the
config loader is designed so reload is possible. Reload adds state transitions:
validate the new config, replace sources, preserve or reset offsets, add and
remove inotify watches, and keep the old config on failure. That is meaningful
complexity for the first release.

## Decision

v0.1 does not reload on SIGHUP. A config change requires a restart. The config
loader is a pure function of file content and returns a fully validated config,
so v0.2 can reuse it for reload. On invalid reload in v0.2, the daemon must keep
running with the old config and log the error.

## Options

- **Defer to v0.2** — Selected. Smaller v0.1 surface; reload can be designed
  against a stable tail engine.
- **Implement in v0.1** — Rejected. Adds watch replacement and offset policy
  decisions before the core tail behavior is proven.
- **Never implement; always restart** — Rejected. The spec keeps reload as a
  desirable v0.2 feature.

## Consequences

- v0.1 users must restart the service after editing the config.
- The loader must stay side-effect free and testable; this is now a design
  requirement, not just a preference.
- v0.2 must define whether unchanged sources keep offsets and what happens to
  removed sources.
