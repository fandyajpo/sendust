# Sendust — Architecture Overview

**Status:** approved design baseline at P02-005; implementation validation remains gated by downstream phases.  
**Authoritative constraints:** [requirements](../product/requirements.md).  
**Detailed design:** [system design](system-design.md).  
**Decisions:** [ADR index](decisions/README.md).

## 1. Shape

Sendust is a two-crate Rust workspace plus a thin TypeScript package.

```text
Consumer application (Node.js / TypeScript)
        │  import { createTransport } from "sendust"
        ▼
ts/src  ── public API, option shape, result/error classes, ESM+CJS entry points
        │  calls generated napi exports
        ▼
crates/sendust-napi  ── Node-API adapter: copy inputs, map errors, own async tasks
        │  calls core types and functions
        ▼
crates/sendust-core  ── validation, message model, MIME, transport state, SMTP orchestration
        │  uses
        ▼
lettre 0.11.23 (Tokio 1 executor) ── SMTP session, TLS, AUTH, DATA
        │
        ▼
Consumer-configured SMTP relay
```

## 2. Why these boundaries

| Boundary | Responsibility | Reason |
| --- | --- | --- |
| TypeScript wrapper | Public names, types, defaults documentation, error classes, module formats | Keeps the native surface small and lets JS-facing ergonomics change without recompiling Rust |
| napi adapter | Convert JS values to owned Rust values; convert core results/errors; schedule async work | Isolates all Node-API and `unsafe`-adjacent code in one crate |
| Rust core | All protocol-independent policy and all SMTP/MIME behavior | Testable without Node; enforces validation, limits, lifecycle, and delivery semantics |
| lettre | SMTP/TLS/AUTH/MIME primitives | Avoids writing and maintaining an SMTP client; pinned and researched |

The TypeScript wrapper is deliberately thin. It validates only public shape and option compatibility that cannot be expressed in generated types. It MUST NOT reinterpret core statuses or implement SMTP/MIME policy. This prevents the two implementations from drifting.

## 3. Two crates, not more

The workspace has exactly two crates in the MVP:

- `crates/sendust-core`
- `crates/sendust-napi`

A separate `sendust-mime` or `sendust-smtp` crate is not justified yet: MIME and SMTP share the message model and validation policy, and splitting them would add versioning and visibility overhead without independent consumers. The boundary that matters is **Node-free core**, which two crates already provide. If a future transport needs to reuse core, the core API is already transport-shaped enough to extract later; that extraction is post-MVP and requires an ADR.

## 4. Data and control flow

1. The caller constructs a transport with relay, TLS, auth, timeout, attachment, and cap options.
2. `send(message)` synchronously prepares an owned native operation, snapshots JS-owned buffers/strings, and reserves the single-send slot; synchronous preparation failures become Promise rejections.
3. The binding schedules the owned operation on napi-managed Tokio 1.
4. Core validates in deterministic order, composes the MIME message, enforces caps, and opens one AsyncSmtpConnection through lettre.
5. lettre performs connect, TLS/required STARTTLS, AUTH, MAIL, one RCPT command per recipient, and one DATA transaction for the accepted-recipient subset.
6. Core maps the final observable state to `accepted`, `rejected`, or `unknown` and closes the session.
7. The binding converts the result or error into a stable JS object; the wrapper returns it unchanged.

No queue, worker, scheduler, queue adapter, or persistence layer exists or is planned in the current scope. No automatic retry exists at any layer. No per-recipient message fan-out exists. Queueing and scheduling remain caller-owned.

## 5. Ownership model

- JavaScript values are valid only during the synchronous portion of the call.
- Every `Buffer`, `ArrayBuffer`, typed array, and string is copied into Rust-owned memory synchronously before async work is scheduled.
- Attachment files are read into bounded Rust-owned buffers after opt-in validation.
- The native transport object owns the core transport and its lifecycle state.
- JavaScript `close()` explicitly transitions the transport; garbage collection never closes it.
- In-flight async work holds its own owned state and cannot borrow JS memory.

## 6. Failure model

Failures are categorized before they reach JavaScript:

| Category | Example | Public behavior |
| --- | --- | --- |
| Validation | Non-ASCII local part, 101 recipients, CRLF in header | `send()` Promise rejection with stable code; transport-option validation in `createTransport()` is synchronous |
| Configuration | Missing host, unsupported auth mode | Synchronous `createTransport()` error before any network attempt |
| Limit | Attachment > 10 MiB, wire > 30 MiB | Rejection before DATA |
| TLS | Self-signed, expired, wrong host | Closed, non-retryable error |
| Authentication | `535` | Closed, non-retryable error |
| Protocol | Explicit SMTP refusal | `rejected` when the relay definitively refuses |
| Protocol | Abrupt close / missing final reply after DATA | `unknown` only when final disposition cannot be established |
| Timeout | Connect/operation/total budget exceeded | Timeout category with elapsed budget |
| Lifecycle | Send after close, concurrent send | `CLOSED` / `BUSY`, no network attempt |
| Internal | Panic or invariant violation | Converted to generic internal error; never unwinds across FFI |

Success means **relay acceptance**. An explicit negative final DATA reply is `rejected`; `unknown` is reserved for a missing/indeterminate final disposition after DATA may have reached the relay. Neither is automatically retried. See [error model](error-model.md).

## 7. Security posture

- Certificate and hostname verification are always enabled; no bypass exists.
- Credentials are held only in owned memory and never serialized into errors or logs.
- Headers and envelope fields reject CRLF and control characters.
- Attachments require explicit filesystem opt-in; only regular files are read; no URLs.
- Caps bound recipients, attachment bytes, wire size, and close duration.
- Panics do not cross the FFI boundary; recoverable failures use `Result`.

## 8. Deployment shape

The npm package is a root package plus platform packages selected by OS/CPU/libc. Consumers install no Rust toolchain. Native artifacts are built in CI, tested on the declared matrix, and published with provenance. See [distribution strategy](../engineering/distribution-strategy.md).

## 9. What the architecture deliberately excludes

- Connection pooling and keep-alive.
- Automatic retry, backoff, or dead-letter handling.
- Per-recipient independent sends.
- SMTPUTF8.
- AbortSignal as a required mechanism.
- URL or streamed attachments.
- Additional transports (SES, SendGrid, etc.).
- Templates, queues, tracking, or analytics.

Their classifications are recorded individually in the [feature matrix](../planning/feature-matrix.md). Queue/job orchestration and queue adapters are `HELD`, not post-MVP commitments; reconsideration requires a separate product decision.
