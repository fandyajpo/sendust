# Release gate — 1.2.0

**Decision:** APPROVED — project owner, 2026-11-23. The owner set DKIM as the
third item of the [next-cycle backlog](../planning/next-cycle-backlog.md) and
asked for that release.
**Release:** `@sendust/ts@1.2.0`, dist-tag `latest`.
**Candidate commit:** `421ee0283b78e1461718c326c56cbc40ea722d87`, the commit the
published facade was built from.
**Related:** [release notes](../releases/1.2.0.md),
[ADR-0024](../architecture/decisions/0024-dkim-signing.md),
[dependency policy](../engineering/dependency-policy.md) §2,
[publication checklist](../engineering/publication-checklist.md) §8.

## 1. Scope

One feature: optional DKIM signing (RFC 6376) in the core, plus the transport
option that switches it on and the append-only error code an unusable key
produces.

Minor is the correct bump under the versioning policy: a new optional option and
a new error code are additive. Nothing that works on `1.1.1` changes behaviour
when `dkim` is absent.

## 2. Why this release exists

Every comparable client signs — Nodemailer, Upyo, the hosted SDKs — and a team
that relays through its own MTA had no way to sign with Sendust at all
([competitive landscape](../research/competitive-landscape.md) §7). XOAUTH2 in
`1.1.0` closed the authentication gap; this closes the deliverability one.

## 3. Candidate checklist

| Check | Evidence |
| --- | --- |
| Owner approval recorded | This record, 2026-11-23 |
| Contract | `dkim` is optional and additive; one new code, `CONFIG_INVALID_DKIM_KEY`, appended to `ErrorCode::ALL` and to the documented taxonomy |
| Rust suite | `cargo test --workspace --all-targets`: 194 unit, 7 `tests/dkim.rs`, every existing suite green |
| Rust lint and format | `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` |
| TypeScript | `tsc`, Biome lint and format, 76 Vitest tests |
| Independent verification | `dkimpy` verifies a signature produced by the core against the fixture's public key (`True`) |
| Signing conformance | `tests/dkim.rs` re-derives the body hash and the signed header block with its own canonicalization and verifies with the raw RSA primitive |
| Wire cap | `delivery` tests prove the cap covers the signature and that an over-cap signed message never reaches the port |
| Error secrecy | `DkimSigner`'s `Debug` renders `<redacted>`; the key never appears in an error or a result |
| Version consistent | `scripts/check-versions.mjs` at 1.2.0 |
| Documentation guards | `npm run check`: architecture, docs, doc-claims, versions all PASS |
| Native artifacts | CI `Native artifact` and `Native artifact (musl)` jobs on the candidate commit |
| Facade hash | to be produced by the `Facade hash` dispatch on the candidate commit; never from a laptop |

## 4. Binary verification

`release/1.2.0/approved.json` pins the approved commit, the dist-tag `latest`, the
facade hash `be29213daeee25a13ac034f3ec8aaad9c808bb90bc00e3da8f7155be10005334`
produced by the `Facade hash` job on that commit (job 137491718675), and all seven
platform entries. No byte-reproducibility claim: each target is identified by the
approval bundle's own checksums, as in `1.1.1`. `publish-release.mjs` refuses to
publish a facade whose digest differs from the pinned value, which is how `1.1.0`
caught a macOS-computed hash
([reproducible build](../engineering/reproducible-build.md) §7).

A second candidate was prepared for the npm-page metadata — `homepage`, the issue
tracker, keywords and a README written as the package page — at `4a74817` with
facade `a93f5ef0…` (job 137505719905). Its publish run stopped on
`@sendust/ts-linux-x64-gnu@1.2.0 already exists; refusing to replace it`: `1.2.0`
had already been published from the first candidate, and npm never replaces a
version. That metadata therefore ships in `1.2.1`, and this record and the
approval bundle name the bytes that are actually on the registry.

## 5. Publication

**Executed 2026-11-23.** The CI `publish` job published `@sendust/ts@1.2.0` and all
seven platform packages over npm trusted publishing. What the registry records:

| Check | Value |
| --- | --- |
| `@sendust/ts@1.2.0` | present; `latest` → `1.2.0`, `next` still `1.0.0` |
| `npm pack @sendust/ts@1.2.0` | `be29213daeee25a13ac034f3ec8aaad9c808bb90bc00e3da8f7155be10005334`, the approved facade hash |
| Registry shasum | `b31be1908f703fe57e61d2be42a98752749262e4`, 42 files, 166 977 bytes unpacked |
| Publisher | `GitHub Actions` with an OIDC trusted-publisher configuration; no npm token |
| Provenance | `attestations.provenance.predicateType` = `https://slsa.dev/provenance/v1` |
| Platform packages | all seven resolve `1.2.0` |

A later publish attempt on the npm-page candidate failed at the first platform
package with `already exists; refusing to replace it`, which is the registry
working as designed: a published version is immutable, so the metadata went to
`1.2.1` instead.
