# Release gate — 1.0.1

**Decision:** APPROVED — project owner approval, 2026-11-21. The owner approved
the patch release in this chat after the projected improvement was reported.
**Release:** `@sendust/ts@1.0.1`, dist-tag `latest`.
**Related:** [release notes](../releases/1.0.1.md),
[real-relay analysis](../qa/real-relay-optimization.md),
[story](../releases/story.md).

## 1. Scope

A patch release with two changes, neither of which alters the public contract:

- demand-driven concurrency in the delivery engine (`ts/src/engine.ts`);
- `RSET` only for an abandoned envelope, and no reuse after `421`
  (`crates/sendust-core`).

Patch is the correct bump under the [versioning policy](../engineering/versioning-policy.md):
`1.0.0` stays valid and installable, no result, code, option, or limit changed,
and a consumer can upgrade without touching code.

## 2. Approval checklist

| Check | Evidence |
| --- | --- |
| Owner approval recorded | This record |
| Contract unchanged | `public-api.md`, `error-model.md`, `types.ts` untouched |
| Rust suite, clippy, fmt | CI `Rust workspace` job and a local full run |
| TypeScript typecheck, 71 tests, format | CI `TypeScript` jobs, local run |
| Runtime behaviour on Node.js, Bun, Deno | `tests/runtime/behaviour.mjs`, 16/16 |
| Five native targets | CI `Native artifact` jobs |
| Wire behaviour pinned | `crates/sendust-core/tests/smtp_reuse.rs`, `ts/test/engine.test.ts` |
| Version consistent | `scripts/check-versions.mjs` |
| Rollback plan | Deprecate `1.0.1` and publish a patch; never republish a version |

## 3. Binary verification

`release/1.0.1/approved.json` pins the approved commit, the dist-tag, and the
facade tarball hash. Per-target binaries are identified by the approval bundle's
own `SHA256SUMS`, which the publisher verifies against the packed tarballs; no
byte-reproducibility claim is made for this release.

## 4. Known limitations at approval

- The Gmail comparison has not been re-run: the expected p95 improvement is a
  projection derived from the owner's `1.0.0` measurement.
- Provider quota policies remain a backlog item.
- Publishing still uses the owner's bypass-2FA token until 2027.

## 5. Publication

From the CI `publish` job with `publish` checked. The dist-tag comes from
`approved.json` (`latest`), so no separate tag step is needed. The tag `v1.0.1`
is created on the approved commit and never moved.
