# QG-10 — Reliability Phase Record

**Decision:** APPROVED — project owner review, 2026-11-15  
**Phase:** P10 — Reliability  
**Gate:** PHASE-10  
**Prepared:** 2026-11-15  
**Branch:** `main`  
**Evidence:** the local Rust, checker, and binding commands below, plus the CI run for this record's commit.

This record is the review package for PHASE-10. The implementation was written by
an agent, so the approval in section 6 is the required human review step, not a
self-approval.

## 1. Task evidence

| Task | Status | Evidence |
| --- | --- | --- |
| P10-001 | COMPLETED | `DeliveryProgress` records the furthest stage and the RCPT-accepted recipients and is reset at the start of every send, so a reused transport cannot carry stale state into a later cancellation; `SmtpTransport::with_progress` marks each stage; a close that cancels after `data_body` returns `unknown` for the accepted subset and `TRANSPORT_CLOSED` before it. Race tests cover send/send, send/close, close/send, close/close. |
| P10-002 | COMPLETED | Exact deadline boundaries at 10 s / 30 s / 120 s with epsilon cases and total capping; timeout classification is honest per stage; timeout tests prove no retry; the facade closes a stalled send inside a 1 s budget. |
| P10-003 | COMPLETED | `tests/leaks.rs` proves stable descriptor counts across 20 successes, 50 refused connections, and 20 TLS failures; attachment handles are released; buffers are owned and dropped after serialization. |
| P10-004 | COMPLETED | 26 deterministic fault cases over the loopback fake TLS relay with results, stages, command traces, and cleanup; stalls at every stage, resets, malformed replies, mixed RCPT, final `451`, and post-DATA ambiguity included. |
| P10-005 | COMPLETED | This record, the failure-mode checklist in section 2, the matrix in section 3, and the stress evidence in section 4. |

## 2. Contract failure-mode checklist

| Contract rule | Result | Evidence |
| --- | --- | --- |
| `accepted` only after a positive final DATA reply | PASS | `accepts_a_single_recipient_transaction`; the final reply is the only path that settles recipients `accepted` |
| An explicit refusal is a `rejected` outcome, never an error | PASS | `all_recipients_refused_never_sends_data`, `a_refused_sender_is_a_rejected_outcome`, `explicit_final_data_rejection_is_a_rejected_outcome` |
| Post-DATA uncertainty is `unknown`, never `accepted` or retried | PASS | `missing_final_reply_after_data_is_unknown`, `final_reply_stall_after_data_is_unknown`, `a_cancellation_after_data_reports_unknown_for_the_accepted_subset` |
| No automatic retry anywhere | PASS | Every timeout test asserts exactly one `MAIL`; the adapter has one transaction per call |
| No queue, pool, worker, or background delivery | PASS | No such code exists; one connection per send |
| One active send; `BUSY` fails before any I/O | PASS | `a_second_send_is_busy_and_dials_nothing`; the facade test records exactly one connection |
| Close is explicit, idempotent, and bounded | PASS | Lifecycle tests, facade double close, close during connect, close within a 1 s budget; `close_drain_times_out_without_the_active_send_finishing` |
| Timeouts are finite, honest, and non-overrunning | PASS | Exact boundary tests; per-stage classification; total caps the operation |
| Every terminal path cleans up | PASS | `leaks.rs` loops; handle-release test; no long-lived socket |
| TLS fails closed with no bypass | PASS | Certificate matrix, STARTTLS refusal, untrusted-root failure, no bypass option in the public API |
| Errors never carry credentials, bodies, or paths | PASS | Serialized-error scans in the facade and the core code sweep |

## 3. Fault matrix summary

| Stage | Fault | Result |
| --- | --- | --- |
| Connect | refused | `PROTOCOL_CONNECTION_RESET` |
| Connect | greeting stall | `TIMEOUT_CONNECT`, no `MAIL` |
| Connect | malformed greeting | `PROTOCOL_UNEXPECTED_RESPONSE`, no `MAIL` |
| TLS | untrusted / self-signed / expired / wrong host / incomplete chain | `TLS_CERTIFICATE`, no `MAIL` |
| TLS | STARTTLS not advertised | `TLS_HANDSHAKE`, no `MAIL` |
| Auth | stall | `TIMEOUT_OPERATION` at `auth`, no `MAIL` |
| Auth | reset | `PROTOCOL_CONNECTION_RESET` at `auth`, no `MAIL` |
| Auth | rejection | `AUTH_FAILED`; no mechanism is `AUTH_UNSUPPORTED` |
| MAIL | stall | `TIMEOUT_OPERATION` at `mail` |
| RCPT | stall / intermediate reply | timeout at `rcpt` / `PROTOCOL_UNEXPECTED_RESPONSE` |
| RCPT | `421` | channel closes, no `DATA`, recipients stay `unknown` |
| RCPT | mixed `250`/`550` | one `DATA` for the accepted subset |
| DATA command | stall / reset | timeout at `data_command` / reset; body empty |
| DATA body | final `451` | `rejected` |
| DATA body | missing final reply / final-reply stall | `unknown` |
| Total | staged stalls exceeding the total | `TIMEOUT_TOTAL`, no retry |

## 4. Stress and leak evidence

`mixed_stress_loop_keeps_descriptors_and_outcomes_stable` alternates 50 successful
sends and 50 refused connections after a warm-up and asserts the `/dev/fd` count is
stable within two descriptors, every success is `accepted`, every refusal is
`PROTOCOL_CONNECTION_RESET`, and the relay saw exactly 50 `DATA` commands — no
duplicate transaction.

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
npm --workspace ./ts run typecheck
npm --workspace ./ts run lint
npm --workspace ./ts run format:check
npm --workspace ./ts run test
```

Result: 175 core unit tests, 26 SMTP fault tests, 3 leak probes plus the mixed
stress loop, 9 harness, 6 client, 4 MIME conformance, 6 MIME body fixtures, 6
session, and 35 facade tests pass; every checker passes.

## 6. Decision

**APPROVED, 2026-11-15.** The project owner reviewed this record and its evidence
and approved the PHASE-10 gate. The implementation was written by an agent, so this
approval is the required human review step, not a self-approval: the reviewer is
the project owner, and the review found and fixed one real defect before approval —
send progress carried across sends on a reused transport, now cleared by
`DeliveryProgress::reset` with its own test.

P10-001 to P10-005 are COMPLETED and PHASE-10 is APPROVED. Phase 11 becomes
eligible and owns the security work that this phase's fault matrix and redaction
checks feed into.

Approval covers P10 only. It does not approve P11 or any later gate; a failed
compile, test, checker, or runtime result on `main` reopens this gate.

## 7. Deferred and explicit non-goals

- **DNS failure fixture**: needs a live resolver; the suite stays offline. A
  resolver-level failure currently surfaces through the same connect deadline and
  `PROTOCOL_CONNECTION_RESET`/`TIMEOUT_CONNECT` paths that are covered.
- **JS-level fake TLS relay**: still deferred to P12 QA; the Rust harness covers
  the transaction-level faults.
- **Non-goals, signed for this gate**: no automatic retry, no connection pooling,
  no internal queue, worker, scheduler, or persistence, no delivery guarantee, no
  delivery confirmation, no post-MVP failover or circuit breakers.
