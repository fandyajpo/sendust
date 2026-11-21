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

- Two tests encoded the old single-session behaviour and failed on the original
  candidate commit: `ts/test/transport.test.ts` and `tests/runtime/behaviour.mjs`
  both asserted one connection for two concurrent sends. They were corrected by
  pinning a one-session pool, and the approval and the tag were both moved to
  `cad80fd33afcd92914d1c6d3d91db01749abcb9e`, where CI is green on all five
  targets. No build input changed, so the facade tarball hash is the same one
  measured before the correction. Nothing had been published when the tag moved.
- The Gmail comparison has not been re-run: the expected p95 improvement is a
  projection derived from the owner's `1.0.0` measurement.
- Provider quota policies remain a backlog item.
- Publishing still uses the owner's bypass-2FA token until 2027.

## 5. Publication

**Executed 2026-11-21.** The CI `publish` job ran on the `v1.0.1` candidate and
published `@sendust/ts@1.0.1` and the five platform packages at `1.0.1`. The
dist-tag came from `approved.json` (`latest`), so no separate tag step was
needed. The tag `v1.0.1` points at the approved commit
`cad80fd33afcd92914d1c6d3d91db01749abcb9e` and was not moved afterwards.

Post-publication verification: the registry tarball from
`npm pack @sendust/ts@1.0.1` reproduces the approved facade hash
`004eb54a03656c1a4311b6b0d91f6f7efa6e3b7331992cdd94674e002ac1f56f`, `latest`
resolves `1.0.1`, and a clean install adds two packages and reports
`version 1.0.1`. The full table is in the
[publication checklist](../engineering/publication-checklist.md) §5b.
