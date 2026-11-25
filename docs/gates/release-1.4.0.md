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
| Facade hash | `4e47363beaadb9a3ca7e6a283558a80f0f138a2fc48f9640d6756e25a1a459b8` from the `Facade hash` dispatch on the candidate commit (`efb8867`, job 138935627083); never from a laptop |

## 4. Binary verification

`release/1.4.0/approved.json` pins the approved commit
`efb8867db0ab2889ff7a73135e6f6939827e4e68`, the dist-tag `latest`, the facade
hash `4e47363beaadb9a3ca7e6a283558a80f0f138a2fc48f9640d6756e25a1a459b8`
produced by the `Facade hash` job on that commit (job 138935627083), and all seven
platform entries. No byte-reproducibility claim: each target is identified by the
approval bundle's own checksums, as in `1.3.0`. `publish-release.mjs` refuses a
facade whose digest differs from the pinned value
([reproducible build](../engineering/reproducible-build.md) §7).

## 5. Publication

**Executed 2026-11-25.** The CI `publish` job published `@sendust/ts@1.4.0` and
all seven platform packages over npm trusted publishing, eight packages in one
run, from the job's own `+ <name>@1.4.0` lines and a closing
`published 8 package(s) at 1.4.0 with tag latest`. What the registry records:

| Check | Value |
| --- | --- |
| `@sendust/ts@1.4.0` | present; `latest` → `1.4.0`, `next` still `1.0.0` |
| Registry artifact | the tarball at `dist.tarball` hashes to `4e47363beaadb9a3ca7e6a283558a80f0f138a2fc48f9640d6756e25a1a459b8`, the pinned facade digest; shasum `0398c66bee4ce9096912daa47e5496f04b592008`, 42 files, 175.0 kB |
| Publisher | `GitHub Actions` with an OIDC trusted-publisher configuration; no npm token |
| Provenance | `attestations.provenance.predicateType` = `https://slsa.dev/provenance/v1`, one transparency-log entry per package |
| npm page | `homepage` = `https://sendust-docs.vercel.app`, `bugs.url` = the issue tracker, 18 keywords |
| Platform packages | all seven resolve `1.4.0` |
| Optional dependencies | all seven pinned to `1.4.0` |
| GitHub Release | [v1.4.0](https://github.com/fandyajpo/sendust/releases/tag/v1.4.0), published 2026-11-25, and pages for `v1.3.0`, `v1.2.1` and `v1.2.0` after it |
| Owner settings | the owner reports both remaining publication settings done on 2026-11-25: publishing access on the eight packages, and the old `NPM_TOKEN` secret removed. Neither is observable from this tree |

The `v1.2.0` and `v1.2.1` pages carry GitHub's generated changelog link,
and `v1.3.0` carries the curated notes; the full notes for all four are in
`docs/releases/`. The one gap this record first opened is closed: `v1.4.0` was
created with both boxes cleared, while `v1.3.0` already held *Latest*, so the
badge stayed on `v1.3.0` until the owner edited `v1.4.0` once with **Set as the
latest release** ticked on 2026-11-25. `releases/latest` now returns `v1.4.0`,
which is what a release page created with both boxes cleared cannot do on its own:
it becomes Latest only when it is the newest at creation.

The registry tarball reproduces the pinned digest byte for byte. npm's read path
lagged the job by about five minutes: `npm view …@1.4.0` and `npm pack` answered
`E404` and `ETARGET` while three platform packages already resolved `1.4.0`, and
the facade's manifest, tarball and `dist-tags` followed. That is the same
ingestion lag the `1.2.1` record kept, so the job log remains the authoritative
"it went up".

The job ran on `3e6fa73`, the pin commit one step after the approved candidate
`efb8867`, and it published because a later commit is accepted only while none of
the build inputs changed — which the script compares against the approved commit
before anything goes up.
