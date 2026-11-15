# TypeScript Contract, Lifecycle, and Error Report (P12-003)

**Status:** complete — all suites pass; two scoped gaps recorded
**Date:** 2026-11-15
**Method:** the exact toolchain pins; `npm --workspace ./ts run typecheck`, `lint`, `format:check`, and `test` under Node.js, plus the runtime smoke entry point on Bun and Deno (executed by CI). Every assertion below names the file and test that carries it.

## 1. Executed suites

| Suite | Tests | What it proves |
| --- | --- | --- |
| `ts/test/types.test.ts` | 7 | The public surface matches its documented shape and rejects anything outside it |
| `ts/test/errors.test.ts` | 4 | Every stable code maps to exactly one class and category; redaction holds |
| `ts/test/results.test.ts` | 4 | All three statuses and mixed recipient lists map verbatim |
| `ts/test/limits.test.ts` | 7 | The facade's pre-flight cap logic matches the core |
| `ts/test/transport.test.ts` | 22 | Lifecycle, BUSY, close, attachments, options, and connection failures through the real addon |
| **Total** | **44** | |

`typecheck` covers `src/**` plus the two contract test files and is part of every CI TypeScript job; `lint` and `format:check` pass with no diff.

## 2. Contract and type tests

| Check | Test | Fails when |
| --- | --- | --- |
| Documented result shape | "types the documented result shape" | A status or recipient field changes |
| Transport surface | "types the transport surface" | An option or method is added, removed, or retyped |
| Error fields | "keeps errors catchable with stable, typed fields" | `code`, `category`, or `details` changes type |
| Frozen option/message surface | "keeps the option and message surfaces exactly as documented" | A public key is renamed or widened |
| Unknown keys rejected | "rejects options outside the frozen surface" (compile-time and runtime) | A caller can pass an undocumented key |
| Attachment shapes | "limits attachments to buffer and path shapes" | A URL, stream, or glob shape becomes representable |
| State and subject | "keeps state read-only and the subject required" | `state` becomes writable or `subject` optional |

These are compile-time assertions checked by `tsc`, so a contract-breaking change fails `npm run typecheck` before any test runs.

## 3. Runtime matrix (statuses and errors)

| Status / class | Test | Level |
| --- | --- | --- |
| `accepted` | "maps accepted without adding a delivery claim" | Native result → facade mapping |
| `rejected` | "keeps rejected and unknown verbatim" | Mapping |
| `unknown` | "keeps rejected and unknown verbatim", "keeps mixed recipient statuses in order" | Mapping |
| Undocumented status | "rejects an undocumented native status" | Mapping; fails closed with `SendustInternalError` |
| Every error code | "covers every stable code with its category and class" | Mapping over the documented code list |
| Class hierarchy | "keeps the class hierarchy narrow enough to catch one category" | Mapping |
| Foreign error | "returns null for a foreign error" | Mapping |
| Redaction | "redacts credentials, bodies, and paths from every error" | Every class, both markers |

End-to-end status coverage through a live relay is proven in Rust (`accepts_a_single_recipient_transaction`, `explicit_final_data_rejection_is_a_rejected_outcome`, `missing_final_reply_after_data_is_unknown`). The facade cannot reproduce that locally, because the public API accepts no trust anchor by design (C-07) and a loopback certificate is therefore untrusted; the gap and its owner are recorded in section 5.

## 4. Lifecycle, BUSY, and error behaviour through the addon

| Behaviour | Test | Pass condition |
| --- | --- | --- |
| Close idempotent | "closes idempotently and refuses later sends" | Double close resolves; `state` is `closed` |
| Concurrent close | "closes concurrently without drifting past closed" | Two closes settle; state is `closed` |
| Close during send | "aborts a send in flight and rejects it as TRANSPORT_CLOSED" | The send rejects with `TRANSPORT_CLOSED` |
| Close budget | "closes a stalled send within the close budget" | Close resolves under 1.5 s |
| Send during close | "refuses a send that starts during close" | `TRANSPORT_CLOSED` |
| BUSY | "rejects a second concurrent send with TRANSPORT_BUSY" | `TRANSPORT_BUSY` **and exactly one connection** |
| Refused connection | "maps a refused connection to a protocol error" | Stable protocol code |
| Attachment opt-in | "rejects a path attachment without filesystem opt-in" | `CONFIG_FILE_ACCESS_DISABLED` |
| File caps | "enforces the per-file cap for a path attachment", "enforces the aggregate cap across path attachments" | Limit codes |
| Buffer copy | "copies a buffer attachment before the send returns" | Caller mutation is not observed |
| Detach | "snapshots a buffer before the caller detaches it" | A detached buffer cannot disturb the send |
| Pre-copy cap | "rejects an oversize attachment before copying it" | `LIMIT_ATTACHMENT_BYTES` before any copy |
| Lowered cap | "honours a lowered attachment cap in the facade" | `LIMIT_ATTACHMENT_BYTES` |
| No attachment shape | "rejects an attachment with neither a path nor content" | `VALIDATION_INVALID_ATTACHMENT` |
| Unknown options | "rejects unknown option keys before touching the relay" | `VALIDATION_INVALID_OPTION`, no connection |
| Auth method | "rejects an unsupported auth method" | `VALIDATION_INVALID_OPTION` |
| Validation before I/O | "validates a message before opening a connection" | Stable validation code, no connection |
| Missing From | "reports a missing From as VALIDATION_MISSING_FROM" | `VALIDATION_MISSING_FROM` |
| Version export | "exposes the package version" | String |

Every BUSY and lifecycle assertion that could retry counts connections or relays; no test observes a second connection for one `send()`.

## 5. Runtime matrix

| Runtime | Exact pin | Suite executed | Recorded result |
| --- | --- | --- | --- |
| Node.js | 22.23.5 | `typecheck`, `lint`, `format:check`, full Vitest suite, runtime smoke | Pass in CI |
| Node.js | 24.22.3 | Full Vitest suite | Pass in CI |
| Node.js | 26.13.1 | Full Vitest suite | Pass in CI |
| Bun | 1.4.4 | Runtime-neutral smoke entry point only | Pass in CI |
| Deno | 2.9.8 | Runtime-neutral smoke entry point only (`--allow-ffi --allow-read`) | Pass in CI |

The baseline deliberately keeps Vitest out of Bun and Deno ("runtime compatibility tests must not depend on Vitest running inside Bun or Deno"), so those two run the smoke entry point. The full behavioural matrix across runtimes belongs to P14-004, which owns the runtime × platform gate; no runtime is claimed supported here (C-06, C-35).

## 6. Gaps and dispositions

| Gap | Why | Owner / disposition |
| --- | --- | --- |
| No end-to-end `accepted`/`rejected`/`unknown` in the JS suite | The public API accepts no trust anchor by design, so a loopback TLS relay is untrusted; mapping and core semantics are both proven, but not in one JS test | P12-004 fixture work / R-04; accepted until the JS relay question is settled |
| `typecheck` covers `src` and the two contract test files only | `@types/node` is not a pinned dev dependency, so the runtime suites (which import `node:net`, `node:fs`) are transpiled by Vitest, not typechecked | Owner decision: adding `@types/node` would let `tsc` cover every test file. Not introduced here (dependency policy) |
| No line coverage | No coverage tool is pinned | Recorded in the P12-002 report; owner decision |
| Bun/Deno run the smoke entry point only | Baseline rule above | P14-004 owns the full matrix |

No contract deviation and no high-severity defect was found. The four gaps are scoped, assigned, and do not change the public contract.
