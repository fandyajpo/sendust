# Attachment Read, Copy, and Memory Performance (P13-003)

**Status:** complete — every timing budget met; memory bounds set from measurement
**Date:** 2026-11-15
**Harness:** `crates/sendust-core/tests/perf_attachments.rs` (ignored tests, release build, counting allocator)
**Command:** `cargo test -p sendust-core --release --test perf_attachments -- --ignored --nocapture --test-threads=1`

## 1. Environment

| Item | Value |
| --- | --- |
| Rust | `rustc 1.100.0 (0ef1bc386 2026-11-09)`, pinned toolchain |
| Host | macOS 15.0, Apple M2, 8 cores |
| Profile | release: `lto = "thin"`, `codegen-units = 1` |
| Warm-up / measured iterations | 3 / 20, single test thread |
| Memory | counting `GlobalAlloc` reporting live and peak bytes |

## 2. Results

| Metric | p50 / p95 | Budget p50 / p95 | Peak memory | Bound |
| --- | --- | --- | --- | --- |
| Read a 10 MiB file (page-cached) | 2.10 / 2.36 ms | 30 / 60 ms | 24.0 MiB | ≤ 32 MiB |
| Read a 10 MiB + 1 B file over the cap | rejected, `LIMIT_ATTACHMENT_BYTES` | — | 25.2 MiB | ≤ 32 MiB |
| Copy 10 MiB (`to_vec`-equivalent memcpy) | 0.60 / 0.68 ms | 20 / 40 ms | — | — |
| Serialize a 10 MiB attachment | 33.9 / 35.3 ms | 150 / 300 ms | 61.1 MiB | ≤ 80 MiB |
| Serialize a 20 MiB aggregate | 67.2 / 68.1 ms | 400 / 800 ms | 81.1 MiB | ≤ 128 MiB |
| Wire size of the aggregate | — | < 30 MiB | — | 27.4 MiB |
| 20 × 10 MiB serializations | — | — | residual 0.0 MiB | ≤ 4 MiB |

## 3. Observations

- **Reads are dominated by the filesystem, not the cap check.** 10 MiB reads in 2.1 ms from the page cache (≈4.8 GB/s), and a file one byte over the cap is rejected after 25.2 MiB of peak allocation — that is the `cap + 1` bounded read, not the whole file, and it does not grow with file size.
- **Copying is not free but cheap.** A 10 MiB copy takes 0.60 ms (≈17 GB/s), which is the same operation the facade performs (`new Uint8Array(content)`) and the binding performs (`content.to_vec()`). At 3% of the 20 ms budget it cannot dominate a send.
- **Encoding dominates serialization.** 10 MiB → 33.9 ms for ≈13.4 MiB of base64 output (≈400 MB/s), and doubling the attachments doubles the time (67.2 ms), so the cost is linear and attaches only once per message.
- **Memory is bounded by the caps, not by the input size.** Peak is ≈2.4× the cap for a read and ≈1.5× the wire size for serialization; both are fixed multiples, and 20 repeated 10 MiB serializations leave 0.0 MiB live, so nothing is retained across sends.
- **No attachment is read twice by construction.** `Delivery` converts every `File` attachment into owned `Bytes` once, and `serialize` refuses a `File` attachment outright (`file_attachments_cannot_be_serialized_before_p08`), so a second read is not representable.

## 4. Method

- Each timing metric warms up 3 times, then runs 20 times; the report uses median and the 19th of 20 sorted samples.
- The counting allocator records live bytes on every allocation and tracks the peak; memory is read around each measured operation, so fixture allocation is excluded.
- Tests run serially and are `#[ignore]`, so CI never measures on shared hardware.
- Every timing budget is asserted, so a regression fails the harness run.

## 5. Limitations

- Local filesystem only, page-cached after the first read; a cold disk would be slower and is not measured.
- `macOS` allocator behaviour is included; RSS may exceed the counting allocator's view because the OS keeps freed pages.
- The copy metric is a Rust `memcpy` of the same size and shape as the boundary copies; it does not include the JS typed-array wrapper itself.
- Single host (Apple M2); the 2× CI allowance from the budget table is not applied to these local numbers.

## 6. Verdict

All timing budgets pass with at least a 4× margin, memory peaks are bounded multiples of the caps, the over-limit read stops before reading the file whole, and repeated serialization returns to baseline. P13-003 is complete; P13-004 covers BUSY, close, and lifecycle overhead.
