# Release gate — 1.0.2

**Decision:** AWAITING PROJECT OWNER APPROVAL. Nothing is published, tagged, or
pinned for this candidate yet. The agent does not approve this gate.
**Release:** `@sendust/ts@1.0.2`, dist-tag `latest`.
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

## 2. Candidate checklist

| Check | Evidence |
| --- | --- |
| Owner approval recorded | Not yet — this record is the candidate |
| Contract unchanged | `public-api.md`, `error-model.md`, `types.ts` untouched |
| Rust suite, clippy, fmt | CI `Rust workspace` job |
| TypeScript typecheck, tests, format | CI `TypeScript` jobs; 72 tests locally |
| Runtime behaviour on Node.js | `tests/runtime/behaviour.mjs`, 16/16 |
| Five native targets | CI `Native artifact` jobs |
| New behaviour pinned | `ts/test/transport.test.ts`, "releases an idle session after the reuse window" |
| Version consistent | `scripts/check-versions.mjs` |
| Documentation guards | `npm run check`, `check-secrets.mjs` |
| Rollback plan | Deprecate `1.0.2` and publish a patch; never republish a version |

## 3. Binary verification

On approval, `release/1.0.2/approved.json` pins the approved commit, the
dist-tag, and the facade tarball hash. Per-target binaries are identified by the
approval bundle's own `SHA256SUMS`, which the publisher verifies against the
packed tarballs; no byte-reproducibility claim is made for this release.

## 4. Known limitations at approval

- The owner's Gmail benchmark has not been re-run against `1.0.x`.
- An idle session is now released rather than kept warm, so a sender with gaps
  longer than `delivery.sessionIdleMs` reconnects per burst.
- Publishing still uses the owner's bypass-2FA token until 2027.

## 5. Publication

After approval: pin `release/1.0.2/approved.json`, create the annotated tag
`v1.0.2` on the approved commit, and dispatch the CI `publish` workflow with
`publish` checked. The dist-tag comes from `approved.json` (`latest`), so no
separate tag step is needed.
