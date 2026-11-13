# ADR-0006 — One active send, explicit bounded close, no pool

**Status:** ACCEPTED (architecture baseline; lifecycle tests P04/P09/P10)  
**Date:** 2026-11-13  
**Deciders:** Architecture review (P02-005)  
**Validated by:** P02-003, P04-004, P09-004, P10-001

## Context

A transport library must define concurrency and shutdown precisely. Connection pools and background queues introduce hidden state, unbounded memory, and shutdown ambiguity. For an MVP focused on correctness, a single in-flight send per transport is simpler to reason about and test.

## Decision

- Each transport permits at most one active `send`.
- A concurrent `send` fails immediately with `TRANSPORT_BUSY`; it is never queued or delayed.
- Lifecycle is `open -> closing -> closed`.
- `close()` is explicit, idempotent, and bounded by the close budget (default 125 s).
- Close does not depend on garbage collection, finalizers, or process exit.
- No connection pool, keep-alive reuse, internal queue, worker, scheduler, persistence, or queue adapter exists in the current scope.
- The transport is a direct send API, not a job submission API. Consumers may independently place calls behind their own infrastructure; Sendust does not define or depend on that infrastructure.
- Callers who need concurrency create multiple transports.

## Alternatives

1. **Internal queue.** Convenient but hides latency and memory growth, complicates close, and obscures BUSY semantics. Rejected for the current product direction; no queue abstraction is designed.
2. **Connection pool.** Better throughput, but adds health checks, reuse races, and idle timeouts; post-MVP.
3. **Unrestricted concurrent sends on one transport.** Requires multiplexing or multiple connections with shared auth state; high complexity and unclear lifecycle.

## Consequences

- The state machine is small and testable.
- Throughput per transport is one send at a time; documented clearly.
- Close can deterministically cancel/await the single in-flight operation.
- Pooling remains a post-MVP feature with its own ADR.

## Evidence required

- State-transition tests: send open, send closing, send closed, double close, close during connect/auth/DATA.
- Concurrency test asserting one SMTP transaction and immediate BUSY.
- Forced GC test proving no close.
- Close deadline test.
