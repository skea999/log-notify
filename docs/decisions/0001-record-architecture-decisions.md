# ADR-0001: Record architecture decisions

## Status

Accepted — 2026-11-26. Deciders: project owner, implementation agent.

## Context

`log-notify` starts as a new repository. The source spec records several origin
decisions, but future changes need durable rationale in the repository itself.
Without a record, rejected options are lost and later contributors re-litigate
settled choices.

## Decision

We will record significant architecture decisions as lightweight ADRs under
`docs/decisions/`, numbered zero-padded in kebab-case, newest last. Each ADR has
Context, Decision, Options, Consequences, and Status sections. Accepted ADRs are
superseded, not deleted.

## Options

- **ADRs in `docs/decisions/`** — Selected. Versioned with the code, short, easy
  to review in a pull request.
- **Decisions only in the spec** — Rejected. The spec is a product contract, not
  a decision log; it would grow without structure.
- **External wiki** — Rejected. It drifts from the repository and is not
  available offline.

## Consequences

- Every sticky choice has a discoverable rationale.
- Small documentation overhead per decision; routine choices are skipped.
- The directory name follows the project workflow rule (`docs/decisions/`), not
  the generic `docs/adr/` convention.
