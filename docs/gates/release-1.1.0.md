# Release gate — 1.1.0

**Decision:** APPROVED — project owner, 2026-11-22. The owner reviewed the
candidate, reported CI green on `dd4336b`, and asked to continue; publication is
their dispatch.
**Release:** `@sendust/ts@1.1.0`, dist-tag `latest`.
**Candidate commit:** `dd4336b4d30a50cdc2b2f3d3fba264b470087016`.
**Related:** [release notes](../releases/1.1.0.md),
[ADR-0020](../architecture/decisions/0020-per-platform-native-packages.md),
[ADR-0023](../architecture/decisions/0023-xoauth2-authentication.md),
[platform matrix](../qa/platform-matrix.md),
[versioning policy](../engineering/versioning-policy.md).

## 1. Scope

Two additive features, chosen from the audit's install blockers:

- **Linux musl**: `linux-x64-musl` and `linux-arm64-musl`, built and loaded
  inside a musl container per architecture, packaged with `libc: ["musl"]`
  (`ts/src/loader.ts`, `scripts/pack-platform.mjs`, `scripts/ci-musl.sh`,
  `artifacts-musl` CI job).
- **XOAUTH2**: `AUTH XOAUTH2` in the core, a bearer-token credential in the
  adapter, and `auth.accessToken` in the facade
  (`crates/sendust-core/src/transports/smtp/`, `crates/sendust-napi/src/lib.rs`,
  `ts/src/transport.ts`, `ts/src/types.ts`).

Minor is the correct bump under the [versioning policy](../engineering/versioning-policy.md):
a new platform target and a new optional option are both minor, and `1.0.x`
stays valid and installable.

## 2. Candidate checklist

| Check | Evidence |
| --- | --- |
| Owner approval recorded | This record, 2026-11-22 |
| Public contract | `public-api.md`, `error-model.md`, `types.ts` updated; no existing code, result or default changed |
| Rust suite, clippy, fmt | CI `Rust workspace`, plus a local full run (179 unit tests) |
| TypeScript typecheck, tests, format | CI `TypeScript` jobs; 74 tests locally |
| Runtime behaviour on Node.js | `tests/runtime/behaviour.mjs`, 16/16 |
| Seven native targets | CI `Native artifact` jobs, including both musl rows |
| musl build and load | `scripts/ci-musl.sh` inside `node:22.23.5-alpine`, `ubuntu-24.04` and `ubuntu-24.04-arm` |
| Release candidate bundle | CI `Release candidate bundle` |
| New behaviour pinned | `crates/sendust-core/tests/smtp_session.rs`, `ts/test/transport.test.ts` |
| Version consistent | `scripts/check-versions.mjs` at 1.1.0 |
| Documentation guards | `npm run check`, `check-secrets.mjs` |
| Rollback plan | Deprecate `1.1.0` and publish a patch; never republish a version |

## 3. Binary verification

`release/1.1.0/approved.json` pins the approved commit, the dist-tag, the facade
tarball hash `d19b8516…`, and all seven platform entries. Per-target binaries are
identified by the approval bundle's own `SHA256SUMS`, which the publisher
verifies against the packed tarballs; no byte-reproducibility claim is made for
this release. The publish job fails if any of the seven artifacts is missing, so
a partial musl release cannot be published by accident.

**The facade hash is the CI one.** The first pin carried a hash produced by a
macOS build of the same commit; the publish job rebuilt it on `ubuntu-24.04` and
refused, because `npm pack` output is not byte-identical across operating
systems. The value above is what the publish job's own build produced on
`9fa2f73`, so it is the only honest pin. See
[reproducible build](../engineering/reproducible-build.md) §7.

## 4. Known limitations at approval

- The XOAUTH2 path has no live tenant evidence; it is proven against a scripted
  relay only.
- Bun and Deno are not claimed on musl.
- The Gmail benchmark has not been re-run against `1.0.x`/`1.1.0`.
- Publishing still uses the owner's bypass-2FA token until 2027, and this
  release carries no npm provenance attestation.

## 5. Publication

**Executed 2026-11-22.** The CI `publish` job ran on the `v1.1.0` candidate and
published `@sendust/ts@1.1.0` and all seven platform packages at `1.1.0`. The
dist-tag came from `approved.json` (`latest`), so no separate step was needed.

The first dispatch failed at the facade hash check, because the pin carried a
hash from a macOS build; it was corrected to the CI hash and the second dispatch
published. Nothing reached the registry during the failed run.

Post-publication verification: `npm pack @sendust/ts@1.1.0` reproduces
`d19b85166073e60382f40cddbb6a096208336497812536a3ee2d0d26cd70f2e5`, every one of
the seven platform packages resolves `1.1.0`, and `latest` resolves `1.1.0`
while `next` stays on `1.0.0`.
