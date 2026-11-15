# Ownership, Redaction, and Denial-of-Service Audit (P11-004)

**Status:** audited — the panic-guard note from P11-001 is closed
**Date:** 2026-11-15
**Scope:** buffer ownership across the async boundary, redaction in errors and across the binding, concurrency, deadlines and close, descriptor and memory bounds, and the panic boundary at the Node-API edge.

## 1. Owned memory

| Check | Result | Evidence |
| --- | --- | --- |
| A buffer is copied before `send()` returns | PASS | `copies a buffer attachment before the send returns` |
| A detached buffer cannot affect the send | PASS | `snapshots a buffer before the caller detaches it` |
| The binding copies into Rust-owned bytes | PASS | `attachment()` in `crates/sendust-napi/src/lib.rs` calls `content.to_vec()` before the task is spawned |
| No borrowed JS memory survives into the async task | PASS | `#[napi(object)]` values are converted synchronously; the task owns `OutgoingMessage` and `Vec<u8>` |
| The facade rejects an over-cap buffer before copying | PASS | `rejects an oversize attachment before copying it`, `assertAttachmentBudget` |
| Two copies of a buffer attachment (facade, binding) | Accepted | Bounded by the cap; the facade copy is the mutation contract, the binding copy is ADR-0008's owned-memory boundary |

A path attachment is opened and read inside the core, not in JS, so its bytes never exist as a JS object.

## 2. Redaction

| Surface | Result | Evidence |
| --- | --- | --- |
| Error Display and Debug | Only the code, category, and whitelisted details | `defaults_are_redacted_and_carry_the_code`, `details_are_whitelisted_and_readable`, `every_code_renders_without_a_secret_marker` |
| Credentials | `SmtpCredentials` Debug redacts user and password; an auth failure carries neither | `credentials_are_redacted_and_validated`, `auth_failure_is_an_error_and_never_mails` |
| Attachment paths | Never in an error; only a caller-supplied display name would be, and it is a name by construction | `errors_never_carry_the_path` |
| Body and attachment bytes | No error path carries them; details are limited to host, port, stage, SMTP codes, timeout, elapsed, and recipient | `ErrorDetails` whitelist |
| Binding to JS | A JSON payload with escaped strings; control characters are stripped, so a payload cannot break its own framing | `to_napi` and `escape` in `lib.rs` |
| Logging | There is no logging crate in the workspace; nothing emits a trace in production | `Cargo.toml` dependency list |
| Repository secrets | `check-secrets` scans every tracked text file; the TLS leaf keys are the one documented exception | `scripts/check-secrets.mjs` |

## 3. Bounds

| Bound | Result | Evidence |
| --- | --- | --- |
| One active send, no queue, no second connection | PASS | `a_second_send_is_busy_and_dials_nothing`, `rejects a second concurrent send with TRANSPORT_BUSY` |
| Connect, operation, total, close deadlines are finite and exact | PASS | `connect_boundary_is_exact`, `operation_boundary_is_exact_and_capped_by_the_total`, `total_boundary_is_exact`, the timeout matrix in `smtp_faults.rs` |
| Close is bounded and explicit, never GC-driven | PASS | `closes a stalled send within the close budget`, `close_drain_times_out_without_the_active_send_finishing`, `closes concurrently without drifting past closed` |
| Attachment reads and wire size are hard-capped | PASS | P11-003 evidence; `read_bounded`, `wire_cap_is_inclusive_and_exact` |
| Descriptors do not leak across repeated successes, failures, TLS failures, or mixed stress | PASS | `repeated_successful_sends_do_not_leak_descriptors`, `repeated_failed_connections_do_not_leak_descriptors`, `repeated_tls_failures_do_not_leak_descriptors`, `mixed_stress_loop_keeps_descriptors_and_outcomes_stable` |
| Garbage collection cannot close a transport | PASS by construction | No finalizer, `Drop`, or global registry exists; `close()` is the only transition, and an in-flight send roots the instance until its promise settles |
| A forced-GC run | Not automated | Deferred to P12; the design has nothing for the collector to trigger, and the deferral is recorded as a residual risk |

## 4. Panic boundary

P11-001 recorded one medium note: the binding had no explicit panic guard. This task closes it.

- `Transport::new` and `Transport::state` now carry `#[napi(catch_unwind)]`, so a panic on a synchronous call is caught and rethrown as a JS error instead of unwinding across the FFI boundary.
- `send`, `close`, and every other async method are already settled by napi-rs' runtime: the generated future is polled under `catch_unwind`, and a panic rejects the promise. The binding additionally maps a `JoinError` to `INTERNAL`.
- A panic therefore never reaches JS as a call that never resolves, and never aborts the host process.
- The binding contains no `unwrap`, `expect`, `panic!`, or slice indexing; the only fallibles are `Result` values that already map to a stable code.

## 5. Findings

| Severity | Finding | Decision |
| --- | --- | --- |
| Medium | No explicit panic guard on synchronous binding entry points (from P11-001) | Fixed: `catch_unwind` on the constructor and `state`; async paths proven settled by the runtime |
| Low | No automated forced-GC test | Deferred to P12; the design relies on no finalizer, so the collector has nothing to run |
| Note | A buffer attachment is copied twice | Accepted and bounded by the cap (P11-003) |

No high-severity finding. P11-005 re-runs this audit with the gate evidence.
