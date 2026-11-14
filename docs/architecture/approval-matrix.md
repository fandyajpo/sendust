# Sendust — Architecture Approval Matrix

**Status:** APPROVED at design level; final ADR reconciliation completed 2026-11-13; implementation validation remains gated  
**Gate:** P02-005 / QG-02  
**Date:** 2026-11-13  
**Reviewer:** Independent architecture audit (GPT-6)

## 1. Approved architecture baseline

    Node.js consumer
      -> TypeScript public wrapper
      -> Node-API adapter (synchronous snapshot/reservation)
      -> napi-managed Tokio 1 async execution
      -> sendust-core (Node-free)
      -> native SMTP client (Sendust codec, rustls, Tokio)
      -> configured SMTP relay

Exactly two Rust crates:
- sendust-core: validation, message model, MIME, attachment policy, lifecycle, SMTP orchestration, results/errors.
- sendust-napi: Node-API conversion, synchronous owned-input snapshot, lifecycle reservation, Promise/task adaptation, result/error conversion.

The TypeScript wrapper is the only public package surface. It exposes createTransport, send, close, types, and stable error classes. It does not implement SMTP or MIME policy.

## 2. Decisions frozen for implementation

| Area | Approved decision | ADR / source | Implementation and test gates |
| --- | --- | --- | --- |
| Crate boundary | Exactly two crates; core has no Node linkage | ADR-0001 | P03-001, P04-005, P05-005 |
| SMTP engine | Sendust-owned native client (codec, parser, capabilities, AUTH, STARTTLS, state machine); no third-party SMTP engine | ADR-0017; supersedes ADR-0002 | P06-001..P06-005 |
| TLS feature graph | rustls `ring` provider + `webpki-roots` through `tokio-rustls`, TLS 1.2 floor; no third-party SMTP engine in production | ADR-0003, ADR-0017 | P06-002, P06-004 Cargo tree |
| TLS policy | TLS 1.2 minimum; explicit WebPKI roots; hostname+chain verification; implicit TLS or required STARTTLS only; no bypass/fallback | ADR-0003 | P06-002, P11-002 |
| Async runtime | napi 3.16.2 + napi-derive 3.7.0 + napi-build 2.6.0; features napi8 + async; managed Tokio 1; no second runtime | ADR-0004 | P05-001, P05-003, P14-004 |
| Rust MSRV | At least 1.88 per pinned napi 3.16.2 source; P03 pins the build toolchain | ADR-0004 | P03-001, P03-002, P05-001 |
| Recipient policy | Observe each RCPT; ordinary refusals are recipient-level; 421 stops session; accepted subset receives one DATA transaction; recipient accepted only after final 2xx | ADR-0005 | P06-003, P06-004, P09-002 |
| Delivery outcomes | Positive final DATA=accepted; explicit negative final reply=rejected; indeterminate result after message transmission=unknown | ADR-0005, ADR-0013 | P06-004, P09-002, P12-002 |
| Error channel | Operational failures are stable typed errors; SMTP delivery outcomes are result values | ADR-0013 | P04-001, P05-004, P09-003 |
| Lifecycle | One active send; immediate BUSY; explicit open→closing→closed; bounded idempotent close; no GC close | ADR-0006 | P04-004, P05-003, P09-004, P10-001 |
| Timeouts | Connect ≤10s, operation ≤30s, total ≤120s, close ≤125s; values lowerable only; connect/operation ≤ total ≤ close; outer connect deadline covers DNS/greeting/EHLO/TLS | ADR-0007 | P04-004, P10-002 |
| Memory ownership | Synchronous N-API snapshot into Rust-owned buffers before async scheduling; no borrowed JS memory across await | ADR-0008 | P05-002, P11-004 |
| Resource caps | 100 recipients; 10 MiB per attachment; 20 MiB raw aggregate; 30 MiB wire | ADR-0009 | P04-003, P08-003, P13-003 |
| MIME | Core owns structure/policy/serialization; vetted primitives may encode individual values | ADR-0010 | P07-001–P07-005 |
| Distribution | Root npm package plus per-platform optional packages; no consumer Rust toolchain or postinstall download | ADR-0011 | P14-002–P14-004 |
| Module formats | ESM and CommonJS via one shared implementation; exact layout finalized in distribution phase | ADR-0012 (accepted design baseline; packaging validation P14) | P14-002, P14-004 |
| Attachments | Filesystem access opt-in; bounded regular files only; no URL/stream attachment | ADR-0014 | P08-001–P08-005, P11-003 |

## 3. SMTP state and recipient matrix

| SMTP event | Overall result | Recipient result |
| --- | --- | --- |
| Connection/TLS/AUTH fails before envelope | Promise rejects with operational error | No SendResult |
| MAIL FROM explicit 4xx/5xx | rejected | All recipients rejected at envelope level |
| RCPT ordinary 4xx/5xx for one recipient | Continue with other recipients | That recipient rejected |
| RCPT 421 | Stop; do not send DATA | Overall rejected; no recipient accepted as a message |
| All RCPT recipients rejected | rejected; do not send DATA | All recipients rejected |
| Some RCPT accepted, final DATA 2xx | accepted | RCPT-accepted recipients accepted; refused recipients rejected |
| DATA command refused before body | rejected | RCPT-accepted recipients rejected; prior RCPT refusals remain rejected |
| Final DATA 4xx/5xx | rejected | All recipients rejected |
| Message transmission starts, final disposition cannot be observed | unknown | RCPT-accepted recipients unknown; explicit RCPT refusals remain rejected |
| No DATA bytes sent because cancellation/close before body | Promise rejects as closed/cancelled | No delivery outcome |

One SMTP transaction per send. No per-recipient message fan-out.

## 4. N-API and TypeScript call contract

To satisfy both immediate BUSY and copy-before-async, the native boundary is split internally:
1. beginSend(message) runs synchronously, validates public shape, copies every Buffer/typed array/string into owned Rust values, and reserves the transport's one-send slot.
2. executeSend(operation) schedules async SMTP work on the napi-managed Tokio runtime and returns a Promise.
3. The public TypeScript send() composes these operations and converts synchronous preparation failures into Promise rejections.
4. beginClose() synchronously transitions the state to closing and signals cancellation.
5. waitClosed() awaits the active operation and bounded cleanup; public close() composes it.

These are internal native methods, not additional public API features. They introduce no queue or deferred-send behavior.

## 5. Explicit non-goals

No queue, worker, scheduler, persistence, queue adapter, automatic retry, backoff, connection pool, keep-alive, per-recipient message fan-out, SMTP server, hosted service, campaign manager, or provider REST integration.

## 6. Open implementation validations (not architecture blockers)

| Validation | Required proof | Gate |
| --- | --- | --- |
| Production dependency set | `cargo tree -e normal` shows no third-party SMTP engine; lettre is a dev-only oracle | P06-004, P06-005 |
| SMTP protocol behavior | Scripted loopback relay for RCPT mix, DATA 354, final 250/451, dropped final reply | P03-004, P06-003, P06-004 |
| TLS | Five negative certificate fixtures; mandatory hostname/chain verification | P06-002, P11-002 |
| Native memory | Buffer mutation/detachment test; owned-copy audit | P05-002, P11-004 |
| Runtime lifecycle | Exact Node.js/Bun/Deno versions from the toolchain baseline; forced GC where supported, explicit close, process exit | P05-003, P14-004 |
| Timeout behavior | Injected-clock tests for all budgets and coarse boundaries | P10-002 |
| File safety | Open-then-fstat, no-follow where supported, file-type and growth fixtures | P08-002, P11-003 |
| MIME | Independent parser conformance and wire-size parity | P07-005 |

## 7. Final review and approval boundary

This matrix was re-reviewed against the final text and status of every P02-scoped ADR after source/API reconciliation. It is the canonical approved design snapshot for implementation. The pre-QG-02 reconciliation window is now closed; subsequent material changes require a superseding ADR.

QG-02 approves the design contract and implementation handoff only. It does not certify compilation, runtime compatibility, SMTP correctness, TLS fixture results, MIME conformance, or package artifacts. Any downstream implementation gate may block or supersede an ADR if executable evidence contradicts this design.

## 8. Architecture Amendment A01 — multi-runtime support

A01 extends this design baseline without replacing the two-crate core architecture.

| Area | Amended decision | Required validation |
|---|---|---|
| Runtime targets | Node.js, Bun, Deno are required MVP targets | Blocking Sendust CI on exact versions; P05/P14 |
| Native binding | One Node-API 8 addon remains the default | Actual addon load/async/lifecycle tests on all runtimes |
| TypeScript wrapper | Runtime-neutral API; isolated native loader | P03/P05/P09 |
| Deno | Local node_modules and explicit --allow-ffi documented | Clean install and runtime test |
| Platforms | Linux x64/ARM64 glibc, Linux x64 musl, macOS x64/ARM64, Windows x64 MSVC candidates | Build, clean install, load and fake-SMTP tests |
| Claims | Runtime × OS × CPU × ABI/libc intersection only | P14 support matrix and blocking CI evidence |

ADR-0015 is accepted as a design amendment. It does not claim Bun, Deno, or any new platform has passed Sendust runtime tests. See [A01 gate](../gates/architecture-amendment-a01.md).


## 9. Architecture Amendment A02 — modular monolith

ADR-0016 is accepted as a design amendment. Sendust uses a modular monolith inside the existing two-crate Rust workspace and one TypeScript package. Core module responsibilities are explicit; domain modules are isolated from lettre/SMTP and runtime bindings; SMTP is the sole MVP adapter behind a narrow internal delivery contract; no public plugin registry or additional crate is introduced. P03/P04/P06/P12 must provide executable dependency, ownership, adapter, and test evidence. This approval is design-level only.
