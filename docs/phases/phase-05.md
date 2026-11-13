# Phase 05 — Binding

**Phase status:** NOT STARTED  
**Phase dependency:** Phase 04  
**Companion references:** [system design](../architecture/system-design.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md)

## Objective

Expose the Rust core through a minimal Node-API 8 binding while preserving ownership, error, lifecycle, and runtime guarantees.

## Scope

In scope: `crates/sendust-napi/src`, copied Buffer inputs, napi-managed Tokio 1 async tasks, explicit close, stable error/status mapping, and binding tests. Out of scope: TypeScript ergonomics, MIME features, pooling, retries, AbortSignal, and release packaging.

## Tasks

### P05-001 — Define Node-API 8 binding surface

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P04-005
- **Description:** Map the approved core transport, message, send, close, result, and error operations to Node-API 8 exports without moving protocol policy into napi. Pin napi 3.16.2, napi-derive 3.7.0, and napi-build 2.6.0; enable napi8 and async explicitly. Specify synchronous owned-input preparation versus asynchronous execution and object lifetime rules.
- **Evidence criteria:** Export table maps every method, input, output, error, and core call. No SMTP behavior is implemented in the binding. Type signatures are reviewed against P02-002.

### P05-002 — Implement owned Buffer and string conversion

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P05-001
- **Description:** Convert JS Buffers, strings, arrays, and options into Rust-owned values before any async yield. Reject invalid encodings and preserve exact bytes for bodies and attachments. Never retain borrowed JS memory.
- **Evidence criteria:** Mutation and detachment immediately after invocation leave sent bytes unchanged. Heap ownership review shows no borrowed Buffer in a future or transport state.

### P05-003 — Implement async task and lifecycle adapter

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P05-001, P04-004
- **Description:** Run core futures through napi’s Tokio 1-managed runtime, retain explicit transport ownership, implement idempotent close and bounded drain, and map overlapping sends to immediate `BUSY`. Do not create a custom runtime or GC-dependent finalizer behavior.
- **Evidence criteria:** Node tests cover close during connect/DATA, double close, send after close, concurrent send, forced GC, and process exit. Close completes or reports deadline failure by 125s.

### P05-004 — Implement stable result/error conversion

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P05-002, P05-003
- **Description:** Convert core statuses and machine-readable errors into stable JS objects/errors without leaking credentials, message content, or internal stack data. Preserve overall `accepted`, `rejected`, `unknown`; recipient `accepted`, `rejected`, `unknown`; `BUSY`, timeout, and closed semantics.
- **Evidence criteria:** Golden mapping tests cover every core error and result. Redaction tests prove credentials and body bytes are absent from errors.

### P05-005 — Binding phase gate

- **Size:** S
- **Status:** NOT STARTED
- **Prerequisites:** P05-001, P05-002, P05-003, P05-004
- **Description:** Verify Node-API 8 compilation, core isolation, ownership, lifecycle, and mapping against the architecture contract on candidate Node 22/24/26.
- **Evidence criteria:** Binding tests pass on every available candidate; unsupported candidates are documented. Core still builds without Node. All five tasks have evidence and no API drift.

## Technical implementation details

The binding is only an adapter: TypeScript calls napi, napi calls core, and core calls lettre. Pin napi 3.16.2/napi-derive 3.7.0/napi-build 2.6.0; explicitly enable napi8 and async; use napi-managed Tokio 1. Copy every Buffer before async execution. One active send returns `BUSY`; no queue, pool, retry, or AbortSignal requirement. Explicit close transitions `open -> closing -> closed` and drains to 125s; GC is never a shutdown mechanism. Defaults remain connect 10s, operation 30s, total 120s. Preserve relay-accepted success, explicit final SMTP rejection, and genuinely unknown post-DATA disposition; never promise delivery or per-recipient independent sends.

## Dependencies

All tasks require the Phase 04 gate; P05-002/003/004 chain from P05-001 as shown. Phase 06 and Phase 08 consume the binding only after P05-005.

## Risks and edge cases

Borrowed memory, runtime ownership mistakes, finalizer races, Node-version differences, error serialization drift, and accidental protocol logic in napi are the primary risks. A forced GC must not close or corrupt an active transport.

## Testing requirements

Use local fake SMTP/TLS fixtures for success, mixed RCPT, explicit final DATA `451` rejection, missing-final-reply-after-DATA unknown, TLS failures, resets, and delays. Add Buffer mutation/detachment, concurrent `BUSY`, explicit close, close deadline, double close, send-after-close, redaction, and Node 22/24/26 lifecycle tests. No external network or production credentials.

## Acceptance criteria

- Node-API 8 adapter compiles and calls only core APIs.
- Buffers are copied before async work.
- Lifecycle and error/result mappings are stable and tested.
- Core remains Node-independent; no queue, pooling, retry, or GC close exists.

## Deliverables

Binding exports, ownership conversion layer, lifecycle adapter, result/error mapping, and candidate-Node test report.

## Completion gate

`PHASE-05` is approved only when P05-005 proves the adapter preserves all core semantics across candidate runtimes and does not introduce Node linkage into core.

## Status

NOT STARTED.
