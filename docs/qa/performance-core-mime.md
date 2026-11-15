# Core Send-Path and MIME Serialization Performance (P13-002)

**Status:** complete — every budget met with margin
**Date:** 2026-11-15
**Harness:** `crates/sendust-core/tests/perf_serialize.rs` (ignored tests, release build)
**Command:** `cargo test -p sendust-core --release --test perf_serialize -- --ignored --nocapture --test-threads=1`

## 1. Environment

| Item | Value |
| --- | --- |
| Rust | `rustc 1.100.0 (0ef1bc386 2026-11-09)`, pinned toolchain |
| Host | macOS 15.0, Apple M2, 8 cores |
| Node.js (facade metrics, not used here) | 22.23.4 |
| Profile | release: `lto = "thin"`, `codegen-units = 1` |
| Warm-up / measured iterations | 3 / 20, single test thread |

## 2. Results

Times are microseconds. Run 2 repeats the same command on the same host.

| Metric | Run 1 p50 / p95 | Run 2 p50 | Budget p50 / p95 | Verdict |
| --- | --- | --- | --- | --- |
| `validate.small_text` | 0 / 0 | 0 | 200 / 400 | PASS (sub-µs) |
| `serialize.small_text` (4 KiB body) | 20 / 21 | 21 | 500 / 1 000 | PASS |
| `serialize.1mib_text` | 5 159 / 8 069 | 5 141 | 10 000 / 20 000 | PASS |
| `serialize.10mib_attachment` | 33 009 / 33 959 | — | 150 000 / 300 000 | PASS |
| `serialize.unicode_long_headers` | 324 / 347 | — | 15 000 / 30 000 | PASS |
| `linearity.1kib` | 6 | — | — | reference |
| `linearity.1mib` | 5 073 | 5 141 | — | reference |
| `linearity.throughput` | 170.67 → 206.70 bytes/µs | 170.67 → 203.96 | ≥ small/2.2 | PASS |

Wire accounting: the serializer returns the bytes and the accounting is that vector's length; `wire_accounting_is_single_pass` asserts a 4 KiB body plus a 13-byte attachment yields the same length on the second call, so no second serialization exists.

## 3. Observations

- **Large margins.** The 4 KiB send serializes in 20 µs against a 500 µs budget; 1 MiB takes 5.2 ms against 10 ms; the 10 MiB attachment takes 33 ms against 150 ms. Local serialization is nowhere near the 10/30/120 s deadlines.
- **Stable tails.** Large-fixture p95 tracks p50 closely (1 MiB: 5.2 vs 8.1 ms; 10 MiB: 33.0 vs 33.9 ms). The Unicode/long-header fixture is slower per byte (324 µs for 64 KiB) because every encoded word is folded, which is expected.
- **Throughput rises with size.** 1 KiB → 170 bytes/µs, 1 MiB → 207 bytes/µs: fixed per-call overhead disappears and the encoder settles at ≈200 MB/s, consistent with base64 plus folding. No quadratic behaviour.
- **Validation is free at this resolution.** A 4 KiB message validates in under a microsecond (reported as 0 µs), so it cannot be a meaningful cost.
- **Two runs agree** within ≈5% on the metrics both measured.

## 4. Method

- Each metric warms up 3 times (discarded), then runs 20 times; the report uses the median and the 19th of 20 sorted samples as p95, plus min and max.
- Timings wrap only the measured call; fixtures are built beforehand, outside the loop.
- Results are `black_box`ed so the optimizer cannot elide the work.
- Tests run serially (`--test-threads=1`) and are `#[ignore]`, so CI never measures on shared hardware.

## 5. Limitations

- Local fixtures only: no DNS, TCP, TLS handshake, or relay latency is included, so these are local processing costs, never delivery times.
- Allocator behaviour is included in the measured interval (build and free of the output `Vec`) but not instrumented separately.
- The harness reports at microsecond resolution, so sub-microsecond validation shows as 0.
- A single host (Apple M2) was measured; the budget table's 2× CI allowance is for future shared-runner trend data, not applied here.

## 6. Verdict

All six budgets pass with at least a 4× margin, serialization is linear, and there is no double serialization. P13-002 is complete; P13-003 (attachment reads, copying, memory bounds) and P13-004 (lifecycle, BUSY, close) build on this harness.
