# ADR-0006: Keep the ISC license

## Status

Accepted — 2026-11-26. Deciders: project owner.

## Context

The source spec recommends dual MIT/Apache-2.0, the Rust ecosystem convention,
and asks for a decision before the first publish. The repository template already
carries an ISC license with copyright holder skea999. The project owner chose to
keep it during initialization.

## Decision

We will license `log-notify` under ISC, copyright skea999. The existing
`LICENSE` file stays. No dual-license arrangement is added.

## Options

- **ISC** — Selected. Short, permissive, already present, no relicensing work.
- **Dual MIT/Apache-2.0** — Rejected by the owner. Ecosystem-friendly, but adds
  two files and a relicensing step with no current need.
- **MIT only** — Rejected. Similar terms to ISC; no reason to replace the
  existing file.
- **Apache-2.0 only** — Rejected. Adds patent and notice obligations that the
  owner does not need for this project.

## Consequences

- Consumers get a simple permissive license.
- The project differs from the common Rust dual-license convention; packagers
  may ask about it.
- Changing the license later requires the copyright holder's decision and, if
  there are outside contributors, their consent.
