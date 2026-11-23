# Release gate — 1.1.1

**Decision:** APPROVED — project owner, 2026-11-23. The owner asked for this
patch to validate the npm trusted-publisher configuration before it expires on
2026-11-25, and ran the `Facade hash` dispatch so the pin comes from CI.
**Release:** `@sendust/ts@1.1.1`, dist-tag `latest`.
**Candidate commit:** `1f17ece033618753676d9b736152227854cc1a90`.
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
| Owner approval recorded | This record, 2026-11-23 |
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
hash `e6a76a0f…` produced by the `Facade hash` job on `1f17ece` (job
137410204743), and all seven platform entries. No byte-reproducibility claim:
each target is identified by the approval bundle's checksums.

## 5. Publication

**Executed 2026-11-23.** The CI `publish` job published `@sendust/ts@1.1.1` and
all seven platform packages over npm trusted publishing. The registry records
`GitHub Actions` as the publisher with an OIDC configuration, and the package
carries a SLSA provenance attestation; no npm token was used.

Two dispatches failed first, both in the publish job's own guard and both before
any npm command that could publish:

1. the guard refused a `NODE_AUTH_TOKEN` that `actions/setup-node` exports for
   the npmrc it writes from `registry-url`;
2. the same guard fired again after the variable was pinned empty at job level,
   because setup-node runs later and overwrites it.

The fix was to drop `registry-url` from the publish job: npm's default registry
is already `registry.npmjs.org`, so nothing exports a token and npm takes the
OIDC path. The empty pin and the guard stay as backstops. Nothing reached the
registry during the failed runs.

Post-publication verification: `npm pack @sendust/ts@1.1.1` reproduces
`e6a76a0fc70ed5015377ad961e83df7fac34fc829c0c6d0af5279ad82b90c0bb`, all seven
platform packages resolve `1.1.1`, and `latest` resolves `1.1.1` while `next`
stays on `1.0.0`.
