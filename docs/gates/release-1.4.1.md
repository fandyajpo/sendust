# Release gate — 1.4.1

**Decision:** APPROVED — project owner, 2026-11-27. After the Rust performance
audit reported its findings and their fixes, the owner asked for the release.
**Release:** `@sendust/ts@1.4.1`, dist-tag `latest`.
**Candidate commit:** the `release: prepare 1.4.1` commit on `main`; recorded in
`release/1.4.1/approved.json` once the CI facade hash exists.
**Related:** [release notes](../releases/1.4.1.md),
[1.4.0 gate record](release-1.4.0.md),
[performance audit](../audits/2026-11-27-rust-performance-audit.md),
[performance budget](../qa/performance-budget.md),
[improvement roadmap](../../audit/code-audit/IMPROVEMENT_ROADMAP.md) §3.4.

## 1. Scope

| Finding | Change | Layer |
| --- | --- | --- |
| P-01 | `push_base64` reuses one line buffer instead of one `String` per 57-byte chunk | core |
| P-02 | `preflight` returns the measured size, so the wire and every part buffer are reserved once | core |
| P-03 | `normalize_body` and `normalized_len` walk bytes in runs rather than characters | core |
| P-04 | An ASCII header value is folded without a copy, and the prefix is pushed in place | core |
| P-05 | `push_multipart` pushes the three boundary segments instead of a `format!` | core |
| P-06 | `Body::Single` holds a `&'static str` content type instead of a copied `String` | core |
| P-07 | `read_bounded` reserves the file's own length, capped by the limit and `take` | core |
| P-08 | The DKIM body canonicalization uses one buffer and a reused line buffer | core |

Patch is the correct bump: every change is allocation and copy work inside the
core. No public API, result, error code, limit or default changes, and the
emitted bytes are identical, which the MIME fixtures, the byte-exact serializer
tests, the DATA transparency tests and the independent DKIM verifier prove by
passing unchanged.

## 2. Why this release exists

The audit measured a 1 MiB message at 5.3 ms, a 10 MiB attachment at 33.8 ms
with a 61 MiB peak, and a DKIM body that allocated a buffer per line. The fixes
remove that work without touching the wire, so the release is a patch a caller
can take without reviewing a contract change.

The two larger candidates stay out of this release: the base64 `simd` feature
changes the pinned feature set in the
[dependency policy](../engineering/dependency-policy.md) §2 and needs evidence on
all seven targets, and streaming composition needs an ADR.

## 3. Candidate checklist

| Check | Evidence |
| --- | --- |
| Owner approval recorded | This record, 2026-11-27 |
| Contract | No source of truth changed; `public-api.md`, `error-model.md`, `types.ts` and the results are untouched |
| Rust suite | `cargo test --workspace --all-targets --locked` — 14 suites, no failure |
| Rust lint and format | `cargo clippy --workspace --all-targets --locked -- -D warnings`, `cargo fmt --all -- --check` |
| TypeScript | `tsc`, Biome lint and format, 79 Vitest tests against the rebuilt addon |
| Documentation guards | `npm run check` PASS at 1.4.1 |
| Secret scan | `node scripts/check-secrets.mjs` |
| Performance | All four harnesses in [the budget](../qa/performance-budget.md) §4 at the candidate commit; the printed metric lines are in §3a |
| Native artifacts | CI `Native artifact` and `Native artifact (musl)` jobs on the candidate commit |
| Facade hash | from the `Facade hash` dispatch on the candidate commit; never from a laptop |

## 3a. Performance evidence

Run at the candidate commit `5c3423a` on the macOS reference host with the pinned
toolchain, release profile (`lto = "thin"`, `codegen-units = 1`), and
`--test-threads=1`. Each line is `metric, p50, p95, min, max, iterations` in
microseconds, or the named unit where the line says one. Every value is inside
its budget in [the performance budget](../qa/performance-budget.md) §2.

```text
cargo test -p sendust-core --release --test perf_serialize -- --ignored --nocapture --test-threads=1
serialize.1mib_text, 3200, 5070, 2288, 6148, 20
linearity.1kib, 3, 3, 3, 3, 20
linearity.1mib, 2387, 2524, 2207, 3069, 20
linearity.throughput_bytes_per_us, 341.33, 439.29
validate.small_text, 0, 0, 0, 0, 20
serialize.small_text, 9, 9, 9, 9, 20
serialize.10mib_attachment, 30820, 35563, 30117, 36138, 20
serialize.unicode_long_headers, 167, 188, 138, 228, 20
```

```text
cargo test -p sendust-core --release --test perf_attachments -- --ignored --nocapture --test-threads=1
copy.10mib, 1449, 2093, 20
read.10mib_file, 1515, 1951, 20
read.10mib_file.peak_mib, 10.0
read.over_limit.peak_mib, 10.0
repeat.20x10mib.residual_mib, 0.0
serialize.attachment_10mib, 33275, 36798, 20
serialize.attachment_10mib.peak_mib, 27.4
serialize.attachment_20mib, 64356, 92654, 20
serialize.attachment_20mib.peak_mib, 54.7
serialize.attachment_20mib.wire_mib, 27.4
```

```text
npm run build:native && node tests/perf/lifecycle.mjs
busy.rejection, 17.8, 93.4, 9.3, 102.2, 20
close.idle, 13.7, 30.0, 10.6, 319.8, 20
close.during_send, 17.6, 20.2, 15.9, 20.7, 20
close.cycles, 1000, descriptor_delta, 0
Node v22.23.5: lifecycle probe passed
```

```text
npm run build:native && npm --workspace ./ts run build && node tests/perf/facade.mjs
facade.send_copy_10mib, 594.1, 1044.7, 476.7, 1201.1, 20
Node v22.23.5: facade copy probe passed
```

The two runs of the same harness recorded before the candidate commit moved the
1 MiB text and the 10 MiB read metrics by more than 20%, so those two are
recorded as ranges in the [release notes](../releases/1.4.1.md), not as single
values. The peaks and the residual memory are stable across all three runs.

## 4. Binary verification

`release/1.4.1/approved.json` pins the approved commit
`24bd21a3c47237a9cdf16d079c528f7d17830f59`, the dist-tag `latest`, the facade
hash `814bf6c1c0c6b755667cf874c4f125ee72437fc7d3212667a8ea58c54d2e8e67`
produced by the `Facade hash` job on that commit (job 140607675601, run
46028597942), and all seven platform entries. No byte-reproducibility claim: each
target is identified by the approval bundle's own checksums, as in `1.4.0`.
`publish-release.mjs` refuses a facade whose digest differs from the pinned value
([reproducible build](../engineering/reproducible-build.md) §7).

The hash was produced at `24bd21a`, the evidence commit one step after the
prepare commit `5c3423a`. That step changed documentation only, so the build
inputs are byte-identical to the prepare commit's — which is exactly the
comparison `publish-release.mjs` makes before it publishes from a later commit.

## 5. Publication

To be recorded here once the CI `publish` job has run.
