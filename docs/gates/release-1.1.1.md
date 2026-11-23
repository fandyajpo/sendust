# Release gate — 1.1.1

**Decision:** AWAITING PROJECT OWNER APPROVAL.
**Release:** `@sendust/ts@1.1.1`, dist-tag `latest`.
**Related:** [release notes](../releases/1.1.1.md),
[publication checklist](../engineering/publication-checklist.md) §8,
[reproducible build](../engineering/reproducible-build.md) §7.

## 1. Scope

One change: the published manifests declare `repository.url`, which npm trusted
publishing requires. The platform manifests copy the field from
`ts/package.json` in `scripts/pack-platform.mjs`.

Patch is the correct bump: no API, result, code, limit or default changed.
`1.1.0` stays valid and installable.

## 2. Why this release exists

The trusted-publisher configuration created on 2026-11-23 must complete its first
successful publish **before 2026-11-25** or it expires. A patch validates it now,
ships the required field, and keeps the deadline away from a feature release.

## 3. Candidate checklist

| Check | Evidence |
| --- | --- |
| Owner approval recorded | This record, once approved |
| Contract unchanged | `public-api.md`, `error-model.md`, `types.ts` untouched |
| Rust suite, clippy, fmt | CI `Rust workspace` |
| TypeScript typecheck, tests, format | CI `TypeScript` jobs; 74 tests locally |
| Runtime behaviour on Node.js | `tests/runtime/behaviour.mjs`, 16/16 |
| Seven native targets | CI `Native artifact` jobs |
| manifest field | `repository.url` in `ts/package.json`, in the prepare-publish manifest, and in a packed platform package |
| Version consistent | `scripts/check-versions.mjs` at 1.1.1 |
| Facade hash | from the `Facade hash` CI job, never a laptop |

## 4. Binary verification

`release/1.1.1/approved.json` pins the approved commit, the dist-tag, the facade
hash produced by the `Facade hash` job, and all seven platform entries. No
byte-reproducibility claim: each target is identified by the approval bundle's
checksums.

## 5. Publication

Dispatch `publish` with the `distTag` input empty and `hash` unchecked, once the
facade hash is pinned. This is the first trusted-publishing run: the job takes an
OIDC token instead of a stored token, and npm attaches a provenance attestation.
After it succeeds, the checklist §8 step 3 applies — disallow tokens on the eight
packages and revoke `NPM_TOKEN`.
