# Performance Budget and Measurement Method (P13-001)

**Status:** defined — budgets fixed before measurement
**Date:** 2026-11-15
**Scope:** every locally measurable cost the MVP owns: MIME serialization, wire-size accounting, buffer copying, attachment reads, BUSY rejection, close and deadline overhead, and repeated lifecycle cycles. Out of scope: real-relay latency, throughput claims, pooling, and retry.

Budgets are set **before** the benchmarks run, so P13-002…P13-004 test them rather than define them. A budget that later proves unreachable is either remediated or explicitly revised with rationale in P13-005; it is never silently relaxed.

## 1. Measurement method

| Element | Rule |
| --- | --- |
| Build | `--release` (`lto = "thin"`, `codegen-units = 1`); no debug assertions |
| Fixtures | Local, deterministic, in-process. No network, no real relay, no credentials |
| Warm-up | 3 iterations, discarded |
| Measured iterations | 20 per metric |
| Reported values | median and p95 (p95 = the 19th of 20 sorted samples); min and max recorded for context |
| Timing | `std::time::Instant` around the measured call, after all inputs are allocated |
| Multi-run stability | The report records two full runs; a metric that moves more than 20% between runs is marked unstable and not used as a gate value |
| Environment capture | `rustc -V`, host OS/version, CPU model and core count, RAM, Node.js version for facade metrics, and the release profile |
| Harness | Ignored Rust tests (`#[ignore]`) run with `cargo test --release -- --ignored --nocapture`, so no benchmark dependency is added. Facade metrics use a standalone `node` script against the staged addon |
| CI behaviour | CI records medians as trend data only. A gate failure is raised when a median exceeds **2× its budget** (noise allowance) or when a correctness bound (memory, descriptors, connections) is violated at any value |

Protocol latency is explicitly **not** measured: the fake relay is local, so every send is dominated by the client's own work. Results describe local processing cost, never delivery time.

## 2. Budget table

| Metric | Fixture | Percentile | Budget | Rationale |
| --- | --- | --- | --- | --- |
| Validate + serialize, small text | 4 KiB body, 1 recipient | median ≤ 0.5 ms, p95 ≤ 1 ms | 0.4% of the 120 s total budget; must stay noise-level |
| Validate + serialize, 1 MiB text | 1 MiB body | median ≤ 10 ms, p95 ≤ 20 ms | ≈100 MB/s floor for a single-pass encoder |
| Validate + serialize, 1 MiB + 10 MiB attachment | 10 MiB attachment, base64 | median ≤ 150 ms, p95 ≤ 300 ms | 13.3 MiB of output at ≈90 MB/s; below 0.2% of the total budget |
| Wire-size accounting | Runs with each serialization | — | included above | Must not double-scan: the same call reports the byte count |
| Facade buffer copy, 10 MiB | `Uint8Array(10 MiB)` through `send()` | median ≤ 20 ms, p95 ≤ 40 ms | Memory-bandwidth bound; measured, never assumed free |
| Attachment read, 10 MiB | page-cached temp file | median ≤ 30 ms, p95 ≤ 60 ms | ≈350 MB/s read plus the cap check |
| BUSY rejection | second send during an active one | median ≤ 0.1 ms, p95 ≤ 0.5 ms | Fail-fast is the contract; must not dial |
| BUSY connections | same fixture | 0 new connections | Hard invariant, not a budget |
| Close, idle | 1 000 cycles | median ≤ 5 ms, p95 ≤ 10 ms | No I/O to drain |
| Close during a stalled send | 1 s close budget | ≤ budget + 100 ms | Bounded, never unbounded |
| 1 000 open/close cycles | sequential | descriptors ±2, RSS growth ≤ 16 MiB | No timer or descriptor accumulation |
| Repeated sends, 100 × 1 MiB | sequential, same message | RSS within 16 MiB of baseline after the loop | Allocator caching is allowed; retained references are not |
| Serialization scaling | 1 KiB vs 1 MiB vs 10 MiB body | ≤ 2.2× time per 10× body | Proves linearity; catches accidental quadratic work |

## 3. Alignment with the contract

- Every budget is far below the 10 s / 30 s / 120 s / 125 s deadlines, so local work can never be the reason a deadline expires.
- The wire budget is expressed against the 30 MiB cap; a 10 MiB attachment plus a 1 MiB body is the largest single-part fixture that fits comfortably.
- No budget depends on pooling, retry, a queue, or a second connection: each measures exactly one send or one lifecycle operation.
- Memory budgets are bounded by the caps (10 MiB per file, 20 MiB aggregate, 30 MiB wire); no fixture allocates beyond them.
- The 2× CI allowance exists because CI runners are shared and vary; a real regression is expected to exceed 2× and a 20% drift is treated as noise.

## 4. Harness specification

| Harness | Location | Runs |
| --- | --- | --- |
| Core serialization and accounting | `crates/sendust-core/tests/perf_serialize.rs` (`#[ignore]`) | `cargo test -p sendust-core --release --test perf_serialize -- --ignored --nocapture` |
| Attachment read and encode | `crates/sendust-core/tests/perf_attachments.rs` (`#[ignore]`) | same command, `--test perf_attachments` |
| Lifecycle, BUSY, close | `crates/sendust-core/tests/perf_lifecycle.rs` (`#[ignore]`) | same command, `--test perf_lifecycle` |
| Facade copy and BUSY | `tests/perf/facade.mjs` | `node tests/perf/facade.mjs` after `npm run build:native` |

Each harness prints one line per metric in a stable format (`metric, p50_us, p95_us, min_us, max_us, iterations`) so two runs can be compared without parsing prose. This task defines the budget and the harness contract; the harnesses themselves are implemented in P13-002…P13-004.

## 5. Regression thresholds

| Threshold | Value | Where enforced |
| --- | --- | --- |
| Gate budget | the table above | P13-005 report |
| CI noise allowance | 2× budget median | P13-005, documented for future CI use |
| Cross-run instability | > 20% between two runs | The metric is reported but not gated |
| Correctness bounds | memory, descriptors, connections | Always gated, independent of timing noise |

## 6. Limitations

- Local loopback only: no DNS, TLS handshake across a real network, or provider latency is measured. No throughput or delivery-time claim is made.
- Allocator caching means RSS may not return exactly to baseline; the budget allows a bounded delta and distinguishes it from retained references by repeating the loop.
- Shared CI hardware is not a stable measurement environment; budgets are defined for the reference host recorded by each run.
- No pooling or retry exists, so no throughput-per-connection metric is meaningful.
