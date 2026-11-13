# Phase 09 — Public delivery API

**Phase status:** NOT STARTED  
**Phase dependency:** Phase 05, Phase 06, Phase 08  
**Companion references:** [system design](../architecture/system-design.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md)

## Objective

Deliver the stable TypeScript public API in `ts/src` that composes the binding, SMTP, MIME, and attachment capabilities with documented relay-accepted semantics, explicit lifecycle, and no hidden queue or retry behavior.

## Scope

In scope: TypeScript types, transport factory/options, send method, close method, error classes, result types, option validation at the public boundary, and API documentation hooks. Out of scope: adding protocol logic, pooling, retries, AbortSignal requirements, SMTPUTF8, URL attachments, and post-MVP convenience features.

## Tasks

### P09-001 — Define public TypeScript types and transport lifecycle API

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P05-005, P06-005
- **Description:** Define the public `Transport` with open configuration, one `send` operation, and explicit `close`. Expose options for relay, TLS, auth, timeouts, attachments opt-in, and caps, with defaults matching architecture. Types must represent relay-accepted results and recipient accepted/rejected/unknown statuses. Close is required for deterministic shutdown; AbortSignal is optional and not required for MVP.
- **Evidence criteria:** Type declarations compile and include no `any` escape for results/errors. API docs state success means relay acceptance, not delivery, and that one active send returns `BUSY`. Timeout defaults are visible: connect 10s, operation 30s, total 120s, close 125s.

### P09-002 — Implement send composition and result mapping

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P09-001, P06-005
- **Description:** Implement `send` by validating public input, calling napi, and mapping results without reinterpreting core statuses. Compose message body, recipients, and attachments through the binding. Enforce no retry and no per-recipient fan-out. Surface overall `accepted`, `rejected`, `unknown`, recipient accepted/rejected/unknown, and `BUSY` exactly as defined.
- **Evidence criteria:** Tests cover successful relay acceptance, pre-DATA rejection, mixed RCPT ADR behavior, explicit final DATA rejection and genuinely unknown post-DATA disposition, and concurrent `BUSY`. Assert only one SMTP transaction for unknown results. Result objects contain no delivery claim.

### P09-003 — Implement public error classes and redaction

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P09-001
- **Description:** Define stable error classes/codes for validation, limits, configuration, TLS, authentication, protocol, timeout, closed, and busy. Preserve machine-readable fields while redacting credentials, body content, attachment bytes, and local paths where appropriate. Ensure errors are catchable and do not require parsing message strings.
- **Evidence criteria:** Golden tests cover every class/code. Redaction tests search serialized errors for password, body marker, attachment marker, and absolute path; none appear. Error inheritance and `instanceof` behavior are tested.

### P09-004 — Implement explicit close and busy behavior at the public boundary

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P09-001, P09-002
- **Description:** Expose idempotent `close()` with deadline-bounded drain and `open -> closing -> closed` observability. Calls after close fail with a stable closed error. A second concurrent send fails immediately with `BUSY` and never queues. AbortSignal may be accepted only if it maps to the same bounded close semantics; it must not be required.
- **Evidence criteria:** Tests cover double close, send during close, send after close, concurrent send, close during connect/DATA, and close deadline. BUSY asserts no second connection. No queue exists. AbortSignal absence is a passing baseline.

### P09-005 — Public API contract gate and usage examples

- **Size:** S
- **Status:** NOT STARTED
- **Prerequisites:** P09-001, P09-002, P09-003, P09-004
- **Description:** Validate the public API against the frozen contract with compile-time type tests and runnable examples using the fake relay. Document relay-accepted semantics, caps, lifecycle, attachment opt-in, and no-retry behavior. Confirm no hidden feature creep.
- **Evidence criteria:** Type tests pass; examples run locally with fake SMTP/TLS only. Contract checklist covers all statuses, errors, limits, close, BUSY, attachments, and semantics. No pooling, retry, SMTPUTF8, URL attachment, or required AbortSignal appears.

## Technical implementation details

The public API is in `ts/src` and calls the napi binding; it contains no SMTP logic. The call path remains `TS wrapper -> napi -> core -> lettre`. `send` means the relay accepted the message, never delivery. An explicit negative final DATA reply returns `rejected`; return `unknown` only when the final disposition cannot be established after DATA may have reached the relay. Neither is automatically retried. One active send per transport; overlap returns `BUSY` immediately with no queue. Close is explicit, idempotent, bounded by 125s, and never GC-dependent. Timeouts default to connect 10s, operation 30s, total 120s, close 125s and are finite/tunable. Attachments require explicit filesystem opt-in; no URL support. Caps are 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw, 30 MiB wire. Envelope local parts are ASCII, domains IDNA, display/body Unicode; SMTPUTF8 is post-MVP. AbortSignal is optional, not required.

## Dependencies

Phase 09 requires Phase 05, Phase 06, and Phase 08 gates. P09-001 depends on P05-005 and P06-005; P09-002 and P09-004 depend on P09-001 and P06-005; P09-003 depends on P09-001; P09-005 depends on all. Phase 10 consumes P09-005; Phase 15 also consumes Phase 09.

## Risks and edge cases

TypeScript types may accidentally imply delivery or per-recipient success. Reinterpreting core errors in the wrapper can create drift. A convenience retry or queue could violate MVP. AbortSignal may be mistaken for required shutdown. Public option defaults may diverge from core. Error redaction can remove too much diagnostic context or leak secrets.

## Testing requirements

Use fake SMTP/TLS fixtures for success, pre-DATA rejection, mixed RCPT, explicit final `451` rejection and missing-final-reply ambiguity, auth failure, TLS failures, connect/command/total timeouts, and close stalls. Add compile-time type tests, error golden tests, redaction scans, concurrent BUSY, double close, send-after-close, and attachment opt-in tests. Verify no external network calls and no retries by counting SMTP transactions. Run examples on candidate Node versions where available.

## Acceptance criteria

- Public API exposes only the frozen MVP contract and compiles with strict types.
- Send results honestly represent relay acceptance, rejection, unknown, recipient statuses, and BUSY.
- Close and lifecycle are explicit and bounded; no queue or GC dependency.
- Errors are stable, catchable, and redacted.
- No pooling, retry, SMTPUTF8, URL attachment, or required AbortSignal.

## Deliverables

TypeScript declarations, transport implementation, send/result mapping, error classes, close/busy behavior, type tests, examples, and API contract gate report.

## Completion gate

`PHASE-09` is approved only when P09-005 proves the public contract matches the frozen architecture with no semantic drift, no hidden queue/retry, and complete lifecycle/error coverage. Phase 10 and Phase 15 may then consume the API.

## Status

NOT STARTED.
