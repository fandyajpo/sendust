# Decisions

Architectural decision records live in one place only:

**[../architecture/decisions/](../architecture/decisions/)**

This directory is a stable link target for documents that reference `docs/decisions/`. It intentionally contains no duplicated ADR content.

## Index

See the [ADR index](../architecture/decisions/README.md) for statuses, owners, and validation tasks.

## Why one location

Duplicating ADRs creates contradictory sources of truth. The architecture directory owns the decision context, alternatives, and consequences. Planning documents link to ADRs by ID (`ADR-0001`, `ADR-0005`, and so on) and do not restate them.
