# Release gate — 1.3.0

**Decision:** APPROVED — project owner, 2026-11-24. After the source audit of
`1.2.1`, the owner asked for its findings to be fixed rather than recorded and
left, so the fixes ship as a minor release.
**Release:** `@sendust/ts@1.3.0`, dist-tag `latest`.
**Candidate commit:** the `release: prepare 1.3.0` commit on `main`; recorded in
`release/1.3.0/approved.json` once the CI facade hash exists.
**Related:** [release notes](../releases/1.3.0.md),
[1.2.1 gate record](release-1.2.1.md),
[audit report](../../audit/code-audit/AUDIT_REPORT.md),
[public API contract](../architecture/public-api.md),
[publication checklist](../engineering/publication-checklist.md).

## 1. Scope

The audit's fixes, at every layer:

| Finding | Change | Layer |
| --- | --- | --- |
| H-01 | `AUTH` and its challenge responses use their own 12,288-byte bound, not the 512-byte command line; an over-long value is `VALIDATION_INVALID_OPTION` | core |
| H-02 | An abnormal task end reports `INTERNAL` and sets an explicit `failed` phase | binding |
| M-01 | Cancellation progress is reset before composition | binding |
| M-02 | A pooled session is released once, in `finally` | facade |
| M-03 | A session is reused only after a positive `RSET` reply | core |
| M-04 | `MAIL FROM` declares `SIZE`; an over-limit message fails locally | core |
| M-07 | `host` rejects embedded whitespace | core |
| M-09 | `state` and `stats.state` report `failed`; a later send names the cause | facade |
| M-12 | The binding job runs a Node.js matrix; `tsc` checks the whole suite | CI |
| L-12 | `sessionIdleMs: 0` means no idle reuse on both sides | facade, core |

Minor is the correct bump: `TransportState` gains `"failed"`, which is additive
to a published union, so a caller's exhaustive `switch` gains a branch. No
result, error code, limit or default value changes, and no message byte changes
except the `SIZE` parameter a relay's advertisement asks for.

Not in this release, because each is a decision the owner has not made: the
core's dead lifecycle module, `validate_in_order`, the two unproducible error
details and `PROTOCOL_REJECTED`
([roadmap](../../audit/code-audit/IMPROVEMENT_ROADMAP.md) §1.3 and §3).

## 2. Why this release exists

The audit of `1.2.1` found two HIGH and thirteen MEDIUM findings in the shipped
path. The HIGH ones are user-visible: a Microsoft 365 or Google Workspace token
could not authenticate at all, and one internal panic turned a transport into one
that answered `TRANSPORT_BUSY` for the life of the process. Leaving them for a
scheduled release would leave the library wrong about its own contract, which is
why the fixes ship here and the release records them.

## 3. Candidate checklist

| Check | Evidence |
| --- | --- |
| Owner approval recorded | This record, 2026-11-24 |
| Contract | `public-api.md` §6–§7, `error-model.md` (`AUTH_FAILED`, `AUTH_UNSUPPORTED`), `requirements.md` C-09 and the guides now list `failed`; `errors.test.ts` still walks every code |
| Rust suite | `cargo test --workspace --all-targets` |
| Rust lint and format | `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` |
| TypeScript | `tsc` over the whole suite, Biome lint and format, 79 Vitest tests |
| Documentation guards | `npm run check` PASS at 1.3.0 |
| Secret scan | `node scripts/check-secrets.mjs` |
| Native artifacts | CI `Native artifact` and `Native artifact (musl)` jobs on the candidate commit |
| Facade hash | `1953fd075e651f2a5724366e76e30c85862184b932eeb36e1fa9a4839adccb77` from the `Facade hash` dispatch on the candidate commit (`e39b786`, job 138421784504); never from a laptop |

## 4. Binary verification

`release/1.3.0/approved.json` pins the approved commit
`e39b7862c7d4c7839a892f83445800d84f51b662`, the dist-tag `latest`, the facade
hash `1953fd075e651f2a5724366e76e30c85862184b932eeb36e1fa9a4839adccb77`
produced by the `Facade hash` job on that commit (job 138421784504), and all seven
platform entries. No byte-reproducibility claim: each target is identified by the
approval bundle's own checksums, as in `1.2.1`. `publish-release.mjs` refuses a
facade whose digest differs from the pinned value
([reproducible build](../engineering/reproducible-build.md) §7).

## 5. Publication

**Executed 2026-11-24.** The CI `publish` job published `@sendust/ts@1.3.0` and
all seven platform packages over npm trusted publishing, eight packages in one
run, from the job's own `+ <name>@1.3.0` lines and a closing
`published 8 package(s) at 1.3.0 with tag latest`. What the registry records:

| Check | Value |
| --- | --- |
| `@sendust/ts@1.3.0` | present; `latest` → `1.3.0`, `next` still `1.0.0` |
| `npm pack @sendust/ts@1.3.0` | `1953fd075e651f2a5724366e76e30c85862184b932eeb36e1fa9a4839adccb77`, the approved facade hash |
| Registry shasum | `5f861ed7fc4a65a371e98db9ade1b7462b595153`, 42 files, 175.0 kB unpacked |
| Publisher | `GitHub Actions` with an OIDC trusted-publisher configuration; no npm token |
| Provenance | `attestations.provenance.predicateType` = `https://slsa.dev/provenance/v1`, one transparency-log entry per package |
| npm page | `homepage` = `https://sendust-docs.vercel.app`, `bugs.url` = the issue tracker, 18 keywords |
| Platform packages | all seven resolve `1.3.0`, each carrying the same project fields |
| Optional dependencies | all seven pinned to `1.3.0` |

The registry tarball reproduces the pinned digest byte for byte, so the release
carries no caveat about which build went up. `publish-release.mjs` is what makes
that checkable: it refuses a facade whose digest differs from `approved.json`, and
it published here because the registry's `npm pack` reads back the same value.

The job ran on `2a721ba`, the pin commit one step after the approved candidate
`e39b786`, and it published because a later commit is accepted only while none of
the build inputs changed — which the script compares against the approved commit
before anything goes up.
