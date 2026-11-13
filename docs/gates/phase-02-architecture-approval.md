# QG-02 — Architecture Approval Audit

**Decision:** APPROVED — DESIGN BASELINE  
**Phase:** P02 — Architecture  
**Reviewed:** 2026-11-13  
**Reviewer:** Independent architecture audit (GPT-6)  
**Approval artifact:** [Architecture approval matrix](../architecture/approval-matrix.md)

## 1. Gate checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Crate boundary | PASS | Exactly two Rust crates; core is Node-free; dependency direction is TS → napi → core → lettre |
| SMTP dependency | PASS | lettre 0.11.23 selected; lower-level public async API selected for RCPT visibility; no custom SMTP |
| TLS feature decision | PASS (source-reviewed) | Defaults disabled; smtp-transport, tokio1-rustls, ring, webpki-roots; no pool/native-tls; compile gate P03-002 |
| TLS security policy | PASS (design) | TLS 1.2 minimum; WebPKI roots; hostname+chain verification; implicit TLS or required STARTTLS; no bypass/fallback |
| Runtime ownership | PASS (design) | napi 3.16.2 + napi-derive 3.7.0 + napi-build 2.6.0; explicit napi8 + async; managed Tokio 1; no second runtime |
| Delivery status | PASS | Final DATA 2xx accepted; explicit final 4xx/5xx rejected; indeterminate final disposition unknown; no mailbox-delivery claim |
| Recipient semantics | PASS (source/API) | Each RCPT reply observable; ordinary refusals are recipient-level; 421 stops session; one DATA to accepted subset; recipient accepted only after final DATA 2xx |
| Error taxonomy | PASS | Operational failures use stable typed errors; delivery outcomes use result values |
| Lifecycle | PASS (design) | One active send, immediate BUSY, open→closing→closed, explicit bounded close, no GC dependency |
| Timeout model | PASS (design) | Connect ≤10s, operation ≤30s, total ≤120s, close ≤125s; lower-only; connect/operation ≤ total ≤ close; outer connect deadline covers DNS/greeting/EHLO/TLS |
| Memory ownership | PASS (design) | Synchronous N-API snapshot into Rust-owned memory before async scheduling |
| Limits and attachment policy | PASS | 100 recipients; 10/20/30 MiB caps; filesystem opt-in; regular files only; no URL attachments |
| Scope boundary | PASS | No queue, worker, scheduler, persistence, queue adapter, automatic retry, connection pool, or fan-out |
| Traceability | PASS | C-01..C-30 map to ADRs, tasks, and executable validation gates |
| Contradiction audit | PASS | Recipient, DATA, timeout, lifecycle, and product-boundary contradictions resolved across architecture documents |

## 2. Approved ADR set

Accepted for the architecture/design baseline:
- ADR-0001 two crates
- ADR-0002 lettre 0.11.23; low-level async connection API
- ADR-0003 rustls/ring/webpki-roots secure feature graph
- ADR-0004 pinned napi-rs crates, napi8 feature, and managed Tokio 1
- ADR-0005 observable per-recipient RCPT policy
- ADR-0006 one active send and explicit lifecycle
- ADR-0007 finite budgets and coarse stage attribution
- ADR-0008 owned memory
- ADR-0009 hard caps
- ADR-0010 MIME composition boundary
- ADR-0011 platform package distribution strategy
- ADR-0013 stable error/result taxonomy
- ADR-0014 filesystem attachment policy

ADR-0012 (ESM/CommonJS package layout) remains proposed for its P14 decision/validation gate. It does not block the Rust/SMTP architecture freeze.

## 3. Non-blocking implementation gates

The following are deliberately not represented as passed:
- P03-002: compile the exact Cargo feature set and inspect cargo tree.
- P03-004/P06-003/P06-004: execute the local fake SMTP matrix for mixed RCPT, DATA=354, final 250/451, and missing final reply.
- P05-002/P05-003: compile the native binding and test owned-buffer conversion, Promise behavior, Tokio integration, close, GC, and process exit.
- P06-002/P11-002: execute certificate/hostname verification fixtures.
- P07-005: independently parse MIME and verify wire-size parity.
- P08-002/P11-003: execute cross-platform filesystem safety tests.
- P10-002: injected-clock timeout interaction tests.
- P14-004: actual Node/platform runtime support matrix.

The audit environment has Node.js 22.16.0 but does not have rustc or cargo. No Rust compilation or native runtime test was executed. The architecture is approved as a design baseline; downstream gates remain release blockers until their evidence exists.

## 4. Final decision

**QG-02 APPROVED.** The architecture has a coherent, source-backed design with no unresolved decision that prevents repository scaffolding or Rust-core implementation. Implementation must follow the approval matrix. Any failed compile, fake-relay test, TLS fixture, or runtime probe may block the relevant phase and require an ADR amendment; this approval does not waive those tests.

Queue/job orchestration remains HELD outside the current roadmap and must not be introduced through implementation or adapter abstractions.