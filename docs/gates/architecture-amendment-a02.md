# Architecture Amendment A02 — Modular Architecture

**Status:** DESIGN APPROVED; implementation validation NOT RUN  
**Date:** 2026-11-13  
**Decision:** [ADR-0016](../architecture/decisions/0016-modular-monolith.md)

## Scope

A02 formalizes Sendust as a modular monolith. It adds internal module ownership and dependency rules while preserving exactly two Rust crates, the shared Node-API addon, Node.js/Bun/Deno runtime target, SMTP-only MVP transport, and the existing queue deferral.

## Accepted design

- One product/package and one shared native addon.
- Exactly two Rust crates: Node-free `sendust-core` and thin `sendust-napi`.
- TypeScript public facade and runtime-specific native loader.
- Explicit core modules: config, address, message, validation, attachments, MIME, delivery, SMTP transport adapter, lifecycle, errors/results.
- Domain modules do not depend on lettre, SMTP, napi, Node, or runtime APIs.
- SMTP protocol implementation is isolated in `transports::smtp` and uses lettre.
- A narrow internal delivery port may be used by delivery orchestration; no public plugin registry or custom transport API in MVP.
- No speculative empty modules, feature flags, extra crates, microservices, or duplicated business logic.
- Queue/job orchestration, workers, retry scheduling, queue persistence/adapters remain outside scope or HELD as already defined.

## Gate checklist

| Check | Expected | Evidence owner | Status |
|---|---|---|---|
| Crate count | Exactly sendust-core and sendust-napi | P03-001 | NOT RUN |
| Dependency direction | Acyclic; napi -> core; no core -> napi/Node | P03-001/P04-005 | NOT RUN |
| Module ownership | Each module has one documented responsibility | P03-001/P04 | NOT RUN |
| Domain isolation | Domain modules do not depend on lettre/SMTP/napi/runtime APIs | P03/P04 | NOT RUN |
| SMTP isolation | SMTP protocol behavior exists only in transports::smtp | P06/P12 | NOT RUN |
| Port contract | Narrow internal delivery contract, no public plugin framework | P04/P06 | NOT RUN |
| API boundary | TypeScript and napi contain no SMTP/MIME policy | P05/P09 | NOT RUN |
| Tests | Module unit tests plus cross-module fake-SMTP integration tests | P04/P06/P07/P08/P12 | NOT RUN |
| Runtime matrix | Node.js/Bun/Deno share the same business semantics | P05/P14 | NOT RUN |
| Scope guard | No queue, worker, retry scheduler, pool, extra transport, or third crate added | P03/P09/P12 | NOT RUN |

## Approval boundary

A02 is approved at design level only. The gate does not approve implementation, compilation, runtime compatibility, or SMTP correctness. Any implementation that violates the two-crate boundary or module dependency rules blocks the relevant phase gate.
