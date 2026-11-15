# QG-09 — Public API Phase Record

**Decision:** SUBMITTED — awaiting project owner approval  
**Phase:** P09 — Public delivery API  
**Gate:** PHASE-09  
**Prepared:** 2026-11-15  
**Branch:** `main`  
**Evidence:** the local Rust, checker, and binding commands below, plus the CI run for this record's commit.

This record is the review package for PHASE-09. The implementation was written by
an agent, so the approval in section 6 is the required human review step, not a
self-approval. P09-001 to P09-005 remain `IN REVIEW` until that approval.

## 1. Task evidence

| Task | Status | Evidence |
| --- | --- | --- |
| P09-001 | IN REVIEW | `ts/src/types.ts` documents the frozen contract — relay acceptance, three statuses, `BUSY`, timeout defaults (10 s / 30 s / 120 s / 125 s), cap defaults, attachment shapes — and `Transport` documents the lifecycle. `AuthOptions.method` is honoured end to end. Unknown option keys, top-level and nested, are rejected with `VALIDATION_INVALID_OPTION`. |
| P09-002 | IN REVIEW | `ts/src/results.ts` maps native results verbatim; attachments infer their content type from the documented extension map; validation rejects an empty subject and reports a missing From as `VALIDATION_MISSING_FROM`; a second concurrent send returns `TRANSPORT_BUSY`. |
| P09-003 | IN REVIEW | One error class per category, all extending `SendustError`, selected by `fromNative`; all 33 stable codes walked; redaction proven with real sends (password, body marker, attachment path). |
| P09-004 | IN REVIEW | Double close, send during and after close, concurrent `BUSY` with exactly one relay connection, close during connect, plus the core lifecycle suite (drain and `TIMEOUT_CLOSE`). |
| P09-005 | IN REVIEW | This record, the contract checklist in section 2, the surface lock in section 3, and the executed evidence in section 5. |

## 2. Contract checklist

| Contract section | Requirement | Evidence |
| --- | --- | --- |
| 1. Surface | Exactly `createTransport`, `Transport`, result/message/attachment/option types, error classes and `ErrorCategory`, `version`. | `ts/src/index.ts` exports exactly those; no plugin, subclass, stream, or callback API exists. |
| 2. Transport options | Required host/port/secure; optional auth/ehloName/timeouts/limits/attachments; defaults and maxima; unknown keys rejected; no plaintext mode, no verification bypass, no pool/retry/proxy. | Type keys locked by `keyof` equality tests; runtime rejection tested; `createTransport({ secure: "none" })` and `{ pool: 4 }` are compile errors and, at runtime, `VALIDATION_INVALID_OPTION`. |
| 3. Message shape | Recipients across `to`/`cc`/`bcc`; non-empty subject; text or HTML; reserved headers; Date and Message-ID set by Sendust. | Core validation tests; MIME parity tests; `tsc` type tests (subject required). |
| 4. Attachments | Buffer and path forms only; path needs opt-in; regular files only; content-type inference; RFC 2231 filenames; no URL form. | P08 gate record; `infers_a_content_type_from_the_extension`; attachment type tests reject `{ url }`. |
| 5. Send | Resolves accepted/rejected/unknown; per-recipient outcomes; one DATA; unknown after DATA ambiguity; rejects with `SendustError` subclasses; never retries or queues. | `results.test.ts` mapping tests; Rust SMTP fault matrix (20 tests) for RCPT/DATA outcomes; no retry code exists. |
| 6. Close | Idempotent, bounded, `open -> closing -> closed`; send after close rejects; uncertain post-DATA disposition should be `unknown`. | Facade close tests and core lifecycle tests. The post-DATA `unknown` case is deferred to P10-001 (section 7). |
| 7. State | Read-only synchronous `open`/`closing`/`closed`. | Getter only; `transport.state = "closed"` is a compile error. |
| 8. Concurrency | One active send; `TRANSPORT_BUSY` immediately, no I/O; multiple transports for concurrency. | Busy test asserts exactly one relay connection; no queue exists. |
| 9. Module formats | ESM and CJS with identical classes. | Deferred to P14-002/P14-004 (distribution). Both jobs already run the facade tests on three Node versions, Bun, and Deno. |
| 10. Non-goals | No pool, retry, queue, worker, AbortSignal option, stream, URL attachment, DKIM, template, provider options, `verify()`, callback. | Surface lock asserts the exact key sets; type tests reject pooling and a plaintext mode; grep shows no such identifiers in `ts/src`. |
| 11. Compatibility | Additive option fields and new error codes are minor; changed defaults are major. | Error codes are append-only and documented in the error model. |

## 3. Surface lock

`ts/test/types.test.ts` asserts, at compile time, that the keys of
`TransportOptions`, `AuthOptions`, `TimeoutOptions`, `LimitOptions`, and `Message`
are exactly the documented sets, so a new option cannot be added without updating
the contract and the test. The same file rejects pooling, a plaintext security
mode, a non-PLAIN/LOGIN auth method, URL attachments, a mutable `state`, and a
missing subject. `check-architecture` fails the build if the shared TypeScript
ever uses `any`.

## 4. Security review

- TLS verification is mandatory; no option, flag, environment variable, or method
  disables it.
- The CJS/ESM class identity rule is P14 scope; the facade already runs on Node 22,
  24, 26, Bun, and Deno in CI.
- Errors carry stable codes and whitelisted details; credentials, bodies,
  attachment bytes, and absolute paths are absent, verified by serialized-error scans.
- No hidden concurrency: one send per transport, `BUSY` without I/O, close bounded.

## 5. Executed evidence

Run on macOS with the pinned toolchain on 2026-11-15:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
node scripts/check-architecture.mjs
node scripts/check-docs.mjs
node scripts/check-secrets.mjs
npm run build:native
npm --workspace ./ts run lint
npm --workspace ./ts run format:check
npm --workspace ./ts run typecheck
npm --workspace ./ts run test
node tests/runtime/smoke.mjs
```

Result: 169 core unit tests, 9 harness, 6 client, 4 MIME conformance, 6 MIME body
fixtures, 20 SMTP fault, 6 session, and 34 facade/type tests pass; every checker
passes; the Node runtime smoke reports 5 binding checks.

## 6. Decision

**SUBMITTED, not approved.** The project owner must review this record and its
evidence. On approval, P09-001 to P09-005 move to `COMPLETED`, PHASE-09 is
recorded as approved with the date, and P10 becomes eligible.

Approval would cover P09 only. A failed compile, test, checker, or runtime result
on `main` reopens this gate.

## 7. Deferred (recorded, not claimed)

- **Close during DATA and `unknown`.** The contract says an aborted post-DATA send
  should leave the accepted subset `unknown`; `close()` currently rejects with
  `TRANSPORT_CLOSED`. Stage-aware cancellation belongs to P10-001.
- **Runnable JS example against a fake TLS relay.** The Rust harness covers
  accepted/rejected/unknown, and the facade covers lifecycle, validation, and
  mapping, but there is no JS-level end-to-end relay run: the client trusts only
  the bundled webpki roots and the frozen contract exposes no trust-root option.
  Adding one is an ADR-0003 decision, so the runtime example is deferred to P12 QA
  (or P14 if it becomes a distribution concern).
- **ESM/CJS class identity and clean-consumer installs.** P14-002/P14-004.
- **A custom `ehloName` default.** The contract now documents `localhost`, which is
  what the engine uses; a system-hostname default would need a new decision.
