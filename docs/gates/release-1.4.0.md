# Release gate — 1.4.0

**Decision:** APPROVED — project owner, 2026-11-25. After the source audit of
`1.2.1`, the owner chose the delete-or-wire resolution for its four remaining
findings: populate the two documented error details, delete the two modules
nothing called, and deprecate the code no path produced.
**Release:** `@sendust/ts@1.4.0`, dist-tag `latest`.
**Candidate commit:** the `release: prepare 1.4.0` commit on `main`; recorded in
`release/1.4.0/approved.json` once the CI facade hash exists.
**Related:** [release notes](../releases/1.4.0.md),
[1.3.0 gate record](release-1.3.0.md),
[ADR-0025](../architecture/decisions/0025-delete-dead-core-modules.md),
[ADR-0026](../architecture/decisions/0026-populate-error-details.md),
[audit report](../../audit/code-audit/AUDIT_REPORT.md).

## 1. Scope

| Finding | Change | Layer |
| --- | --- | --- |
| M-05 | The core's `lifecycle` module is deleted; the binding owns the state machine, and `SendDeadlines` moves to `config` | core |
| M-06 | `validation` and `validate_in_order` are deleted; precedence is the order the stages run in | core |
| M-10 | `details.recipient` is populated in the `RCPT` loop, and `details.smtpEnhancedCode` wherever a reply supplied one | core |
| M-11 | `PROTOCOL_REJECTED` is deprecated, not deleted, because codes are append-only within a major version | docs, core |

Minor is the correct bump: the core is internal and unpublished, but the
release records an architecture decision (ADR-0025) and a deprecation
(ADR-0026), and neither belongs in a patch.

## 2. Why this release exists

The audit's rule is delete or wire a contract, never keep a third copy of it.
Two modules described rules the binding and the inline stages already enforce,
and two documented details had no producer. This release resolves all four
without changing anything a caller can observe, except that two documented
fields now appear.

## 3. Candidate checklist

| Check | Evidence |
| --- | --- |
| Owner approval recorded | This record, 2026-11-25 |
| Contract | `error-model.md` and the errors guide carry the deprecation; `public-api.md`, `types.ts` and the results are unchanged |
| Rust suite | `cargo test --workspace --all-targets --locked` — 14 suites, 184 core tests |
| Rust lint and format | `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` |
| TypeScript | `tsc`, Biome lint and format, 79 Vitest tests against the rebuilt addon |
| Documentation guards | `npm run check` PASS at 1.4.0, including the architecture checker with both modules gone |
| Secret scan | `node scripts/check-secrets.mjs` |
| Native artifacts | CI `Native artifact` and `Native artifact (musl)` jobs on the candidate commit |
| Facade hash | to be produced by the `Facade hash` dispatch on the candidate commit; never from a laptop |

## 4. Binary verification

Not yet pinned. `release/1.4.0/approved.json` is written once the `Facade hash`
job reports the tarball digest for the candidate commit, and
`publish-release.mjs` refuses a facade whose digest differs from it
([reproducible build](../engineering/reproducible-build.md) §7).

## 5. Publication

Not executed. The `publish` job publishes `@sendust/ts@1.4.0` and the seven
platform packages over npm trusted publishing (OIDC), with no npm token. After it
runs, this section records the result, the tag and the post-publication
`npm pack` digest, and the [publication checklist](../engineering/publication-checklist.md)
gains its record.
