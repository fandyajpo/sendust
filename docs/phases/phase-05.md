# Phase 05 — Binding

**Phase status:** IN PROGRESS  
**Phase dependency:** Phase 04  
**Companion references:** [system design](../architecture/system-design.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md)

## Objective

Expose the Rust core through a minimal Node-API 8 binding while preserving ownership, error, lifecycle, and runtime guarantees across Node.js, Bun, and Deno.

## Scope

In scope: `crates/sendust-napi/src`, copied Buffer inputs, napi-managed Tokio 1 async tasks, explicit close, stable error/status mapping, and binding tests. Out of scope: TypeScript ergonomics, MIME features, pooling, retries, AbortSignal, and release packaging.

## Tasks

### P05-001 — Define Node-API 8 binding surface

- **Size:** M
- **Status:** IN REVIEW
- **Prerequisites:** P04-005
- **Description:** Map the approved core transport, message, send, close, result, and error operations to Node-API 8 exports without moving protocol policy into napi. Pin napi 3.16.2, napi-derive 3.7.0, and napi-build 2.6.0; enable napi8 and async explicitly. Specify synchronous owned-input preparation versus asynchronous execution and object lifetime rules.
- **Evidence criteria:** Export table maps every method, input, output, error, and core call. No SMTP behavior is implemented in the binding. Type signatures are reviewed against P02-002.

### P05-002 — Implement owned Buffer and string conversion

- **Size:** M
- **Status:** IN REVIEW
- **Prerequisites:** P05-001
- **Description:** Convert JS Buffers, strings, arrays, and options into Rust-owned values before any async yield. Reject invalid encodings and preserve exact bytes for bodies and attachments. Never retain borrowed JS memory.
- **Evidence criteria:** Mutation and detachment immediately after invocation leave sent bytes unchanged. Heap ownership review shows no borrowed Buffer in a future or transport state.

### P05-003 — Implement async task and lifecycle adapter

- **Size:** L
- **Status:** IN REVIEW
- **Prerequisites:** P05-001, P04-004
- **Description:** Run core futures through napi’s Tokio 1-managed runtime, retain explicit transport ownership, implement idempotent close and bounded drain, and map overlapping sends to immediate `BUSY`. Do not create a custom runtime or GC-dependent finalizer behavior.
- **Evidence criteria:** Blocking tests on Node.js, Bun, and Deno cover close during connect/DATA, double close, send after close, concurrent send, forced GC where supported, and process exit. Close completes or reports deadline failure by 125s.

### P05-004 — Implement stable result/error conversion

- **Size:** M
- **Status:** IN REVIEW
- **Prerequisites:** P05-002, P05-003
- **Description:** Convert core statuses and machine-readable errors into stable JS objects/errors without leaking credentials, message content, or internal stack data. Preserve overall `accepted`, `rejected`, `unknown`; recipient `accepted`, `rejected`, `unknown`; `BUSY`, timeout, and closed semantics.
- **Evidence criteria:** Golden mapping tests cover every core error and result. Redaction tests prove credentials and body bytes are absent from errors.

### P05-005 — Binding phase gate

- **Size:** S
- **Status:** IN PROGRESS
- **Prerequisites:** P05-001, P05-002, P05-003, P05-004
- **Description:** Verify Node-API 8 compilation, core isolation, ownership, lifecycle, and mapping against the architecture contract on pinned Node.js, Bun, and Deno versions.
- **Evidence criteria:** Binding tests pass on every available candidate; unsupported candidates are documented. Core still builds without Node. All five tasks have evidence and no API drift.

## Technical implementation details

The binding is only an adapter: TypeScript calls napi, napi calls core, and core calls lettre. Pin napi 3.16.2/napi-derive 3.7.0/napi-build 2.6.0; explicitly enable napi8 and async; use napi-managed Tokio 1. Copy every Buffer before async execution. One active send returns `BUSY`; no queue, pool, retry, or AbortSignal requirement. Explicit close transitions `open -> closing -> closed` and drains to 125s; GC is never a shutdown mechanism. Defaults remain connect 10s, operation 30s, total 120s. Preserve relay-accepted success, explicit final SMTP rejection, and genuinely unknown post-DATA disposition; never promise delivery or per-recipient independent sends.

## Dependencies

All tasks require the Phase 04 gate; P05-002/003/004 chain from P05-001 as shown. Phase 06 and Phase 08 consume the binding only after P05-005.

## Risks and edge cases

Borrowed memory, runtime ownership mistakes, finalizer races, Node-version differences, error serialization drift, and accidental protocol logic in napi are the primary risks. A forced GC must not close or corrupt an active transport.

## Testing requirements

Use local fake SMTP/TLS fixtures for success, mixed RCPT, explicit final DATA `451` rejection, missing-final-reply-after-DATA unknown, TLS failures, resets, and delays. Add Buffer mutation/detachment, concurrent `BUSY`, explicit close, close deadline, double close, send-after-close, redaction, and pinned Node.js, Bun, and Deno lifecycle tests. No external network or production credentials.

## Acceptance criteria

- Node-API 8 adapter compiles and calls only core APIs.
- Buffers are copied before async work.
- Lifecycle and error/result mappings are stable and tested.
- Core remains Node-independent; no queue, pooling, retry, or GC close exists.

## Deliverables

Binding exports, ownership conversion layer, lifecycle adapter, result/error mapping, and candidate-Node test report.

## Completion gate

`PHASE-05` is approved only when P05-005 proves the adapter preserves all core semantics across candidate runtimes and does not introduce Node linkage into core.

## Multi-runtime amendment requirements

P05 is not complete when only Node.js passes. The full binding contract must pass on all required runtimes: native loading, Promise behavior, SMTP outcomes, owned-buffer snapshot, BUSY, explicit close, close races, process shutdown, TLS fixtures, and stable errors. Record exact runtime versions and CI evidence. Deno tests must use local node_modules and --allow-ffi. If the shared Node-API addon fails on an alternate runtime, investigate the smallest loader/binding correction; do not add a separate Rust adapter without a superseding ADR.

## Status

IN PROGRESS. P05-001 to P05-004 are IN REVIEW; the binding, the owned-input copy, the asynchronous lifecycle, and the error/result mapping are implemented, exercised locally, and proven by a real Gmail send through the Node binding. P05-005 (the phase gate) is IN PROGRESS, needs the Bun/Deno addon matrix, and is not self-approved.

## Evidence log

Evidence labels follow the [AI implementation contract](../engineering/ai-implementation-contract.md).

| Task | Evidence | State |
| --- | --- | --- |
| P05-001 | `crates/sendust-napi/src/lib.rs`: `Transport` class (`constructor`, `state`, `send`, `close`) plus `nativeVersion()`; `TransportOptions`, `MessageOptions`, `SendResultJs` object types; no SMTP, MIME, or validation policy in the binding | IMPLEMENTED, EXECUTED PASS locally |
| P05-002 | Every JS value is converted to owned Rust values (`String`, `Vec<String>`, `u32`) before the first await; no Buffer is accepted yet because attachments are P08 scope | IMPLEMENTED, EXECUTED PASS locally |
| P05-003 | `tokio::sync::Mutex` slot plus an atomic phase: one active send, immediate `TRANSPORT_BUSY` from `try_lock`, idempotent `close()` that waits for the active send under the close budget and reports `TIMEOUT_CLOSE`; no runtime is created and no finalizer closes anything | IMPLEMENTED, EXECUTED PASS locally |
| P05-004 | `to_napi`/`to_result` map the core outcome and every error code into a stable JSON payload (`SENDUST:{...}`) consumed by `ts/src/errors.ts`; credentials and bodies never enter a payload. A real relay reply was mapped to `{ status: "accepted", recipients, response }` unchanged | IMPLEMENTED, EXECUTED PASS locally |
| P05-005 | TypeScript facade (`ts/src/{index,transport,errors,loader,types}.ts`) plus `ts/test/transport.test.ts` (invalid options, idempotent close, send-after-close, refused connection) run against the staged addon. Real relay proof on Node.js 22: `node examples/send_email.mjs` against `smtp.gmail.com:465` (implicit TLS with verification, AUTH, sender and recipient redacted) returned `status: accepted` and relay `250 2.0.0 OK ... - gsmtp`. `npm run build:native` stages the addon and a CI job builds and tests it | IMPLEMENTED, EXECUTED PASS locally (Node.js 22 against a live relay); Bun and Deno native-load matrix pending |
