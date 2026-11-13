# Phase 10 — Reliability

**Phase status:** NOT STARTED  
**Phase dependency:** Phase 09  
**Companion references:** [system design](../architecture/system-design.md), [reliability strategy](../engineering/reliability-strategy.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md)

## Objective

Harden the public API and transport for deterministic failure behavior under concurrency, timeouts, disconnects, partial ambiguity, and shutdown, without adding pooling or automatic retries. Reliability here means honest, bounded, leak-free behavior rather than guaranteed delivery.

## Scope

In scope: lifecycle races, timeout-budget interaction, close/drain guarantees, resource cleanup, fault injection, idempotence, and failure observability. Out of scope: retry policy, queueing, pooling, high-availability relay failover, delivery confirmation, and post-MVP circuit breakers.

## Tasks

### P10-001 — Implement lifecycle race hardening

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P09-005
- **Description:** Audit and harden all `open -> closing -> closed` transitions under concurrent send, close, and failure. Ensure exactly one active send, immediate `BUSY` without dialing, no send after close initiation, and idempotent close. Define behavior for close arriving while connect, auth, RCPT, or DATA is in flight. No GC-dependent behavior.
- **Evidence criteria:** Deterministic race tests cover send/send, send/close, close/send, close/close, and failure/close at each stage. BUSY tests assert zero additional SMTP connections. Every terminal state is reachable and documented.

### P10-002 — Enforce timeout-budget interaction and bounded close drain

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P10-001
- **Description:** Implement shared-deadline logic so connect 10s, operation 30s, total 120s, and close 125s cannot overrun their intended bounds. Ensure close drains in-flight work up to 125s, then reports a bounded failure without hanging the process. Timeout classification must be honest where stage attribution is observable and aggregate otherwise.
- **Evidence criteria:** Fake-time tests assert exact boundaries for 10s, 30s, 120s, and 125s with epsilon cases. Close during each stage completes or fails by the deadline. No timeout triggers a retry. No unbounded wait or timer leak remains.

### P10-003 — Implement resource cleanup and leak prevention

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P10-001
- **Description:** Ensure sockets, file descriptors, buffers, timers, and async tasks are released on success, validation failure, TLS failure, timeout, reset, abort/close, and process shutdown. Define cleanup ordering and avoid double-close or use-after-close. Attachment buffers must drop promptly.
- **Evidence criteria:** Repeated success/failure loops show stable descriptor and memory counts. Tests cover failure during connect, TLS, auth, DATA, attachment read, and close. No file descriptor, timer, or task remains after each terminal path.

### P10-004 — Build the reliability fault-injection suite

- **Size:** XL
- **Status:** NOT STARTED
- **Prerequisites:** P10-001, P10-002, P10-003
- **Description:** Create deterministic fault fixtures for connection refused, DNS failure, greeting stall, TLS handshake stall, auth stall, RCPT stall, DATA stall, final-response stall, reset at each stage, abrupt close, malformed responses, slow close, and ambiguous post-DATA outcomes. Tests must assert result classification, timing bounds, command traces, and cleanup.
- **Evidence criteria:** Each fault fixture has an expected result, deadline, command trace, and resource-cleanup assertion. Mixed RCPT `250`/`550` and post-DATA `451` are included. No test depends on external network or unbounded sleeps. Suite passes repeatably.

### P10-005 — Reliability phase gate and failure-mode review

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P10-001, P10-002, P10-003, P10-004
- **Description:** Review all failure paths against the contract: accepted only on relay acceptance, unknown after DATA, no retry, no queue, bounded close, honest timeouts, and resource cleanup. Run stress loops and document residual risks that are intentionally out of scope.
- **Evidence criteria:** Gate report includes fault-matrix results, stress-run stability, descriptor/memory evidence, and a signed list of non-goals (no retry, no pooling, no delivery guarantee). Any leak, hang, or duplicate-send finding blocks approval.

## Technical implementation details

Reliability is built on the frozen architecture: one active send per transport, fail-fast `BUSY`, explicit `open -> closing -> closed`, and close drained to 125s without GC. Timeouts are finite and tunable: connect 10s, operation 30s, total 120s, close 125s, with shared deadlines. No pooling and no automatic retries; after DATA an unknown result is not resent. Success means relay acceptance only. TLS remains verified; no fallback. JS buffers are already copied, attachments are bounded regular files, and caps are hard at 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw, and 30 MiB wire. Reliability work must not introduce a queue or per-recipient fan-out. Where P01-003 could not prove exact stages, timeouts are reported at the honest aggregate level.

## Dependencies

Phase 10 requires Phase 09. P10-001 depends on P09-005; P10-002 and P10-003 depend on P10-001; P10-004 depends on P10-001/002/003; P10-005 depends on all. Phase 11 consumes P10-005; Phase 12 later consumes Phase 11.

## Risks and edge cases

Race windows between close and completion, timer leaks, socket half-close, DNS hangs not covered by connect timeout, server that accepts then stalls, duplicate sends after ambiguous failures, descriptor leaks on TLS/auth failure, and cleanup ordering that drops a future at an unsafe point are key risks. A process exit during close must not produce a false success.

## Testing requirements

All tests use loopback fake SMTP/TLS with scripted faults. Required: send/send, send/close, close/send, double close; stalls at connect 10s, operation 30s, total 120s, close 125s; connection refused; DNS failure; TLS handshake stall; auth stall; RCPT stall; DATA stall; final-response stall; reset at greeting/EHLO/AUTH/MAIL/RCPT/DATA/final; abrupt close; malformed reply; slow drain; mixed RCPT; post-DATA `451`; repeated failure loops for leaks; process-exit during send. Assert no retry by counting transactions and no hang beyond deadlines.

## Acceptance criteria

- Lifecycle races have deterministic, documented outcomes.
- All timeout budgets are bounded and non-overrunning.
- Every terminal path cleans up sockets, descriptors, timers, buffers, and tasks.
- Fault suite covers all required fixtures and passes repeatably.
- No queue, retry, pooling, delivery guarantee, or GC-dependent close exists.

## Deliverables

Hardened lifecycle, shared deadline logic, cleanup guarantees, fault-injection suite, stress/leak reports, and reliability gate record.

## Completion gate

`PHASE-10` is approved only when P10-005 proves bounded, leak-free, honest failure behavior across the full fault matrix and the no-retry/no-queue contract remains intact. Phase 11 may begin only after approval.

## Status

NOT STARTED.
