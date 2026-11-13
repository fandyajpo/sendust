# Performance Strategy

**Applies to:** P13, with earlier design constraints from P02/P04/P08.  
**Rule:** do not optimize prematurely. Define budgets first, measure second, optimize third, re-verify correctness last.

## 1. Goals

1. Establish defensible budgets for the MVP send path, MIME composition, attachment handling, and lifecycle operations.
2. Detect regressions before release.
3. Document worst-case memory under the hard caps.
4. Avoid optimization that changes semantics or weakens safety.

## 2. What is measured

| Area | Metric | Notes |
| --- | --- | --- |
| Validation | Time for a representative message | Excludes network |
| MIME composition | Time and allocated bytes for text, alternative, mixed | Scale with body and attachment size |
| Attachment handling | Read + copy time and peak memory | Buffer and file paths separately |
| FFI boundary | Conversion overhead for strings and buffers | Node ↔ Rust |
| Send path | Overhead excluding relay latency | Fake relay with zero artificial delay |
| BUSY rejection | Time to fail a concurrent send | Must be immediate, no I/O |
| Close | Time to drain idle and active transports | Bounded by 125 s |
| Peak memory | Worst-case caps | 100 recipients, 20 MiB raw attachments, 30 MiB wire |

## 3. What is not measured as a product claim

- Relay latency, internet throughput, or provider behavior.
- TLS handshake time in real networks.
- Delivery time or mailbox acceptance.
- Concurrency beyond one send per transport (MVP has no pool).

## 4. Budget process

1. **P13-001** defines budgets and the measurement method before any benchmark runs.
2. Budgets are expressed as absolute limits and relative regression thresholds.
3. Budgets are approved at the performance gate entry.
4. Measurements use a fixed environment, repeated runs, and reported median plus spread.
5. A regression beyond the threshold blocks QG-13 unless the cause is understood and the budget is explicitly revised with evidence.

## 5. Measurement method

- Rust benchmarks use `criterion` with warmup and repeated samples.
- Memory is measured with an allocator hook or platform tooling; the method is recorded.
- Node-level overhead is measured with a harness that calls the addon directly against a fake relay.
- No benchmark contacts a real relay or external network.
- Benchmarks are run on the same machine class and toolchain as the recorded baseline.
- Results include toolchain version, commit, OS, CPU, and flags.

## 6. Optimization order

Only after measurement:

1. Remove avoidable copies that are not required for ownership safety.
2. Reduce serialization passes (compose once, measure once).
3. Improve attachment read buffering within caps.
4. Reduce allocation churn in header encoding.
5. Revisit MIME encoding choices only if conformance is preserved.

Never:

- Introduce zero-copy borrowed JS memory across async.
- Relax caps to improve throughput.
- Remove validation or verification.
- Add pooling or retry to improve benchmark numbers.
- Trade correctness for speed.

## 7. Memory model

Peak memory is approximately:

```text
owned inputs (strings + buffers)
+ file attachment bytes read into memory
+ serialized wire bytes
+ lettre encoding buffers
+ fixed overhead
```

Caps bound each term. P13-003 records measured peak for worst-case inputs and compares it to the documented expectation.

## 8. Concurrency

MVP concurrency is intentionally one send per transport. Benchmarks must not simulate pooling. If a caller needs concurrency, multiple transports are used; that is documented but not optimized as a single-transport feature.

## 9. Performance acceptance

- Budgets exist and are approved.
- Every measured area is within budget or has an accepted, documented exception.
- Peak memory is bounded and documented.
- No correctness regression after optimization; the full QA suite passes.
- No benchmark depends on external network or secrets.
- Results are reproducible within the recorded spread.

## 10. Reporting

The final report includes:

- Budget table and measured values.
- Environment and method.
- Median, spread, and sample count.
- Peak memory table.
- Known limitations and non-goals.
- Any accepted regression with rationale.
