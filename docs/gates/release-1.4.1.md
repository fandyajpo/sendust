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
| Performance | All four harnesses in [the budget](../qa/performance-budget.md) §4 at the candidate commit; the metric lines are in the [release notes](../releases/1.4.1.md) and the [audit](../audits/2026-11-27-rust-performance-audit.md) §2 |
| Native artifacts | CI `Native artifact` and `Native artifact (musl)` jobs on the candidate commit |
| Facade hash | from the `Facade hash` dispatch on the candidate commit; never from a laptop |

## 4. Binary verification

`release/1.4.1/approved.json` pins the approved commit, the dist-tag `latest`,
the facade hash the `Facade hash` job produced on that commit, and all seven
platform entries. No byte-reproducibility claim: each target is identified by the
approval bundle's own checksums, as in `1.4.0`. `publish-release.mjs` refuses a
facade whose digest differs from the pinned value
([reproducible build](../engineering/reproducible-build.md) §7).

## 5. Publication

To be recorded here once the CI `publish` job has run.
