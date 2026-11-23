# Release gate — 1.2.0

**Decision:** APPROVED — project owner, 2026-11-23. The owner set DKIM as the
third item of the [next-cycle backlog](../planning/next-cycle-backlog.md) and
asked for that release.
**Release:** `@sendust/ts@1.2.0`, dist-tag `latest`.
**Candidate commit:** `4a74817dc6c3f36d5e7c5cdba46be032f8107d20`, the commit that
carries the 1.2.0 tree and the CI facade hash.
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
facade hash `a93f5ef00a139fc88061d82b9ea44f246c4d8b9932b07a692a361acaf130d8a4`
produced by the `Facade hash` job on that commit (job 137505719905), and all seven
platform entries. No byte-reproducibility claim: each target is identified by the
approval bundle's own checksums, as in `1.1.1`. `publish-release.mjs` refuses to
publish a facade whose digest differs from the pinned value, which is how `1.1.0`
caught a macOS-computed hash
([reproducible build](../engineering/reproducible-build.md) §7).

The candidate was pinned twice. The first pin was `421ee02` with facade
`be29213d…`; the npm-page work (homepage, issue tracker, keywords, and a
README written as the package page) changed the published manifest and README,
both of which are part of the tarball, so the pin and the `v1.2.0` tag moved to
the commit that carries them. Nothing had been published at either point.

## 5. Publication

Not executed. The `publish` job publishes `@sendust/ts@1.2.0` and the seven
platform packages over npm trusted publishing (OIDC), with no npm token, and the
registry must record `GitHub Actions` as the publisher and a SLSA provenance
attestation. After it runs, this section records the result, the tag and the
post-publication `npm pack` digest.
