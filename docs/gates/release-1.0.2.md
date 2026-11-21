# Release gate — 1.0.2

**Decision:** APPROVED — project owner approval, 2026-11-21. The owner approved
this patch release in the chat ("lets go") after the audit findings and the
1.0.2 scope were reported.
**Release:** `@sendust/ts@1.0.2`, dist-tag `latest`.
**Candidate commit:** `bc72a6c28edd7464fde3107579fa4fb5fbce5b7c`.
**Related:** [release notes](../releases/1.0.2.md),
[versioning policy](../engineering/versioning-policy.md),
[ADR-0022](../architecture/decisions/0022-session-reuse.md).

## 1. Scope

Two internal fixes found in the repository audit of 2026-11-21:

- the delivery engine releases an idle native session when
  `delivery.sessionIdleMs` elapses (`ts/src/engine.ts`);
- `createTransport` validates `delivery.*` before it creates a native transport,
  and the pool stops re-pooling a session the close path owns
  (`ts/src/transport.ts`).

Patch is the correct bump under the [versioning policy](../engineering/versioning-policy.md):
`Fix behavior without a contract change` — no result, error code, option, limit,
or default changed, and `1.0.1` stays valid and installable.

## 2. Approval checklist

| Check | Evidence |
| --- | --- |
| Owner approval recorded | This record, 2026-11-21 |
| Contract unchanged | `public-api.md`, `error-model.md`, `types.ts` untouched |
| Rust suite, clippy, fmt | CI `Rust workspace` job on the candidate |
| TypeScript typecheck, tests, format | CI `TypeScript` jobs on the candidate; 72 tests locally |
| Runtime behaviour on Node.js | `tests/runtime/behaviour.mjs`, 16/16 |
| Five native targets | CI `Native artifact` jobs on the candidate |
| Release candidate bundle | CI `Release candidate bundle` job on the candidate |
| New behaviour pinned | `ts/test/transport.test.ts`, "releases an idle session after the reuse window" |
| Version consistent | `scripts/check-versions.mjs` |
| Documentation guards | `npm run check`, `check-secrets.mjs` |
| CI on the candidate | `main` at `bc72a6c`: 14 passed, 2 skipped (the manual `publish` and `dist-tag` jobs) |
| Rollback plan | Deprecate `1.0.2` and publish a patch; never republish a version |

## 3. Binary verification

`release/1.0.2/approved.json` pins the approved commit, the dist-tag, and the
facade tarball hash produced by `npm run prepare:publish` on this commit.
Per-target binaries are identified by the approval bundle's own `SHA256SUMS`,
which the publisher verifies against the packed tarballs; no
byte-reproducibility claim is made for this release.

## 4. Known limitations at approval

- The owner's Gmail benchmark has not been re-run against `1.0.x`.
- An idle session is now released rather than kept warm, so a sender with gaps
  longer than `delivery.sessionIdleMs` reconnects per burst.
- Publishing still uses the owner's bypass-2FA token until 2027.

## 5. Publication

Pin `release/1.0.2/approved.json`, create the annotated tag `v1.0.2` on the
approved commit, and dispatch the CI `publish` workflow with `publish` checked.
The dist-tag comes from `approved.json` (`latest`), so no separate tag step is
needed. Nothing has been published at the time this record was written.
