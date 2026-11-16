# QG-13 — Performance Phase Record

**Decision:** PENDING — awaiting project-owner approval  
**Phase:** P13 — Performance  
**Gate:** PHASE-13  
**Prepared:** 2026-11-15  
**Branch:** `main`  
**Evidence:** the three performance reports in `docs/qa/` and the harnesses behind them; CI on the commit that adds this record.

This record is the review package for PHASE-13. The implementation was written by an
agent, so the approval in section 7 is the required human review step, not a
self-approval. Nothing here approves P14 or any later gate.

## 1. Task evidence

| Task | Status | Evidence |
| --- | --- | --- |
| P13-001 | IN REVIEW | [`performance-budget.md`](../qa/performance-budget.md): 12 metrics with method, percentile, threshold and rationale; warm-up/iteration rules, environment capture, 2× CI allowance, harness contract, and explicit non-claims. |
| P13-002 | IN REVIEW | [`performance-core-mime.md`](../qa/performance-core-mime.md): `tests/perf_serialize.rs` — six budgets, all met with ≥4× margin, linear scaling, single-pass accounting. |
| P13-003 | IN REVIEW | [`performance-attachments.md`](../qa/performance-attachments.md): `tests/perf_attachments.rs` with a counting allocator — read, over-limit, copy, serialization, aggregate, and repeated-run memory results. |
| P13-004 | IN REVIEW | [`performance-lifecycle.md`](../qa/performance-lifecycle.md): `tests/perf/lifecycle.mjs` — BUSY, close, concurrency, and descriptor results with every budget asserted. |
| P13-005 | IN REVIEW | This record. |

## 2. Acceptance criteria

| Phase-13 criterion | Result | Evidence |
| --- | --- | --- |
| Performance budgets are defined, measurable, and reproducible | PASS | P13-001 table; every metric prints one stable line and asserts its budget |
| Core send and MIME serialization meet budgets without double work | PASS | P13-002; wire accounting is the returned vector |
| Attachment memory stays bounded and returns to baseline | PASS | P13-003; peaks are bounded multiples of the caps, 0.0 MiB residual after 20 runs |
| BUSY is fail-fast with no queue; close and deadlines stay bounded | PASS | P13-004; 50 concurrent sends → 50 BUSY, one connection |
| Report states limitations and provides regression thresholds | PASS | Section 4 and 5 |

## 3. Consolidated results

| Metric | Measured (p50 / p95) | Budget | Verdict |
| --- | --- | --- | --- |
| Validate, 4 KiB message | < 1 µs | 200 / 400 µs | PASS |
| Serialize, 4 KiB body | 20 / 21 µs | 500 / 1 000 µs | PASS |
| Serialize, 1 MiB body | 5.2 ms / 8.1 ms | 10 / 20 ms | PASS |
| Serialize, 10 MiB attachment | 33.9 / 35.3 ms | 150 / 300 ms | PASS |
| Serialize, Unicode + long headers | 324 / 347 µs | 15 / 30 ms | PASS |
| Serialization linearity | 170 → 207 B/µs (1 KiB → 1 MiB) | ≥ small/2.2 | PASS |
| Read, 10 MiB file | 2.10 / 2.36 ms | 30 / 60 ms | PASS |
| Read, 10 MiB + 1 B over cap | rejected, 25.2 MiB peak | bounded | PASS |
| Copy, 10 MiB | 0.60 / 0.68 ms | 20 / 40 ms | PASS |
| Serialize, 20 MiB aggregate | 67.2 / 68.1 ms, 27.4 MiB wire | 400 / 800 ms, < 30 MiB | PASS |
| 20 × 10 MiB serializations | 0.0 MiB residual | ≤ 4 MiB | PASS |
| BUSY rejection | 24.2 / 93.8 µs, 0 extra connections | 100 / 500 µs | PASS |
| Close, idle | 11.1 / 26.6 µs | 5 / 10 ms | PASS |
| Close during a stalled send | 24.7 / 37.5 µs | ≤ 1.1 s | PASS |
| 1 000 open/close cycles | descriptor delta 0 | ±2 | PASS |
| 50 concurrent sends | 50 BUSY, 1 connection | all BUSY | PASS |

## 4. Regression thresholds

- A metric fails when its median exceeds **2×** the budget (shared-CI noise allowance) or when a correctness bound (peak memory, descriptors, connections) is violated at any value.
- Cross-run instability above **20%** marks a metric as reported-but-not-gated.
- All budgets sit at least two orders of magnitude below the 10/30/120/125 s deadlines, so local work cannot cause a deadline expiry.

## 5. Limitations

- Local loopback only: no DNS, network TLS handshake, or relay latency. These are local processing costs, never delivery times, and no throughput or SLA claim is made.
- Measured on one host (Apple M2, macOS 15.0, rustc 1.100.0, Node 22.23.4); CI records medians as trend data only and never gates on them.
- Close during an established session or during DATA is covered functionally by the P10 close-budget tests, not timed here.
- Allocator behaviour is included; RSS may exceed the counting allocator's view.

## 6. Findings

| Severity | Finding | State |
| --- | --- | --- |
| Note | Initial memory bounds in the harness were guesses and too tight (61 MiB measured for a 10 MiB attachment) | FIXED: bounds set from measurement with documented rationale, not silently relaxed |
| Note | First lifecycle probe set only `closeMs` and was rejected by the core timeout validation | FIXED: the whole timeout set is lowered; the validation did its job |
| Low | No real-network latency data | Accepted limitation of the phase, recorded above |

No high-severity finding, no contract deviation, no budget exceeded.

## 7. Decision

**PENDING.** This record is complete and every budget passes, but PHASE-13 is not
approved until the project owner reviews it and answers with an explicit approval.
The phase stays IN PROGRESS until then, and P14 may not begin.

The review should confirm three points:

1. The budget table and the measured results are the right evidence for NFR-005.
2. The local-fixture limitation is acceptable for the MVP, and no throughput claim
   is made from it.
3. The remaining owner decisions (licence, advisory stage, `@types/node`, coverage
   tool) may stay open until P13/P14 packaging.

## 8. Deferred and non-goals

- Deferred: real-network latency, throughput benchmarks, profiling/allocator flame graphs.
- Non-goals: pooling, retry, queueing, or any optimisation that changes the contract; capacity claims beyond the measured local fixtures.
