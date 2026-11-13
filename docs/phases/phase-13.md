# Phase 13 — Performance

**Phase status:** NOT STARTED  
**Phase dependency:** Phase 12  
**Companion references:** [performance strategy](../engineering/performance-strategy.md), [testing strategy](../engineering/testing-strategy.md), [reliability strategy](../engineering/reliability-strategy.md), [system design](../architecture/system-design.md)

## Objective

Measure and validate MVP performance and resource bounds without changing the no-pooling/no-retry contract. Performance acceptance must focus on bounded memory, predictable latency under local fixtures, and absence of pathological overhead in serialization, copying, and attachment handling.

## Scope

In scope: benchmark design, baseline measurements, memory and descriptor bounds, timeout-overhead validation, attachment/MIME cost, concurrency-rejection overhead, and performance regression thresholds. Out of scope: connection pooling, throughput optimization by adding queues, delivery latency to real providers, and capacity claims beyond measured local fixtures.

## Tasks

### P13-001 — Define performance budgets and measurement method

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P12-005
- **Description:** Define measurable budgets for single-send overhead, MIME serialization, buffer copying, attachment reads, BUSY rejection, and close drain. Specify warm-up, iterations, percentile reporting, environment capture, and tolerance for CI variance. Distinguish protocol latency from local processing cost. Do not set unverifiable real-relay throughput claims.
- **Evidence criteria:** Budget table has metric, method, percentile, threshold, and rationale. Measurement environment is recorded. Budgets align with 10/30/120/125s deadlines and hard caps. No budget depends on pooling or retry.

### P13-002 — Benchmark core send path and MIME serialization

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P13-001
- **Description:** Benchmark validation, address/header encoding, message serialization, wire-size computation, and fake-relay send overhead for text-only and alternative messages. Include Unicode and long-header cases. Separate serialization from network wait and record allocations where feasible.
- **Evidence criteria:** Results report median and tail values against budgets. Serialization remains linear with body size. No double serialization occurs. Regressions above threshold are flagged with evidence.

### P13-003 — Benchmark attachment reads, copying, and memory bounds

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P13-001
- **Description:** Measure per-file and aggregate attachment costs at boundary sizes, including raw read, encoding, serialized wire output, and peak memory. Verify 10 MiB/20 MiB/30 MiB caps prevent unbounded allocation. Measure Buffer-copy cost at the binding boundary and cleanup after send.
- **Evidence criteria:** Peak memory stays within documented bounds for boundary inputs. Over-limit cases fail before excessive allocation where feasible. Buffer copying is measured, not assumed free. Memory returns to baseline after repeated sends. No attachment is read twice.

### P13-004 — Measure lifecycle, BUSY, timeout, and close overhead

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P13-001, P10-005
- **Description:** Measure immediate BUSY rejection with no dial, close during idle and during send, deadline overhead, and repeated open/close cycles. Confirm no queue growth and no timer accumulation. Validate that one active send remains a hard invariant under load.
- **Evidence criteria:** BUSY latency and connection count prove fail-fast behavior. Repeated close cycles show stable descriptors, timers, and memory. Deadline tests show no early expiry or late overrun. No queue exists under concurrent load.

### P13-005 — Performance regression gate and final report

- **Size:** S
- **Status:** NOT STARTED
- **Prerequisites:** P13-001, P13-002, P13-003, P13-004
- **Description:** Consolidate benchmarks into a reproducible report, compare against budgets, and define regression thresholds for future changes. Document limitations: local fake relay, no real-network latency claims, no pooling, no retry. Gate fails on unbounded memory, double serialization/read, queueing, or deadline violation.
- **Evidence criteria:** Report includes methodology, environment, raw results, budget comparison, and limitations. Regression thresholds are stored for CI. No critical budget is exceeded without accepted remediation. Phase 14 final matrix and Phase 16 may consume P13-005.

## Technical implementation details

Performance work measures the architecture-approved path after P02-005: `TS -> napi -> core -> lettre 0.11.23`. There is no pooling and no automatic retry, so benchmarks must not add them. Success remains relay acceptance. One active send returns `BUSY`; concurrency is rejected, not queued. Close is explicit and bounded by 125s; timeouts are connect 10s, operation 30s, total 120s. Caps are 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw, 30 MiB wire. Buffers are copied before async; attachments are opt-in bounded regular files. Serialization must be single-pass and deterministic. TLS uses rustls `ring`/`webpki-roots` with verification always on. Results are local-fixture measurements, not delivery SLAs.

## Dependencies

Phase 13 requires Phase 12. P13-001 depends on P12-005; P13-002 and P13-003 depend on P13-001; P13-004 adds P10-005; P13-005 depends on all. Phase 14 final matrix depends on P12-005 and P13-005 via task dependencies; Phase 16 depends on Phase 13 through its declared inputs.

## Risks and edge cases

CI noise can create false regressions; thresholds need tolerance. Measuring only averages hides tail stalls. Attachment benchmarks may accidentally allocate beyond caps. Buffer-copy cost may dominate small messages. Fake-relay timing can be mistaken for real SMTP performance. Memory may not return to baseline due to allocator behavior; retained references must be distinguished from allocator caching.

## Testing requirements

Use deterministic local fixtures and fixed message sizes: small text, UTF-8, long headers, 10 MiB attachment, 20 MiB aggregate, 30 MiB wire boundary, 100 recipients. Measure serialization, send overhead, Buffer copy, attachment read, BUSY rejection, repeated open/close, and close during send. Run multiple iterations with warm-up; report median and tail. Add leak checks after repeated success/failure loops. Assert no extra SMTP transactions and no queue growth.

## Acceptance criteria

- Performance budgets are defined, measurable, and reproducible.
- Core send and MIME serialization meet budgets without double work.
- Attachment memory stays bounded and returns to baseline.
- BUSY is fail-fast with no queue; close/deadlines remain bounded.
- Report states limitations and provides regression thresholds.

## Deliverables

Performance budget, benchmark harness specification, core/MIME results, attachment/memory results, lifecycle results, regression thresholds, and final performance report.

## Completion gate

`PHASE-13` is approved only when P13-005 shows budgets met or explicitly remediated, bounded memory, fail-fast BUSY, no queue, and no deadline overrun. Phase 14 final matrix and Phase 16 may use the results after approval.

## Status

NOT STARTED.
