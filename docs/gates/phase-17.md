# QG-17 — RC and MVP approval record

**Decision:** P17-002 and P17-003 APPROVED by the project owner, 2026-11-18;
publication (P17-004) authorized and ready to dispatch from CI, nothing published
**Phase:** P17 — RC and MVP approval/publication
**Gate:** PHASE-17
**Prepared:** 2026-11-18
**Candidate:** `0.1.0-rc.1` at commit `a8a71e7b613e0c56bbd48655ba5a2aedaa55afa5`
**Branch:** `main`

This record is the approval package for PHASE-17. No publish command has run and
no artifact has been sent to a registry. The project owner accepted the MVP
(P17-002) and authorized publication (P17-003); the two owner-side prerequisites
are in place and execution needs only the owner's manual workflow dispatch,
recorded in section 6.

## 1. Candidate identity

Verified in the [RC verification record](../qa/rc-verification.md).

| Item | Value |
| --- | --- |
| Version | `0.1.0-rc.1`, agreeing across both manifests, the Cargo workspace, the facade constant, the lockfile, and the changelog |
| Approved commit | `a8a71e7b613e0c56bbd48655ba5a2aedaa55afa5` |
| Verification commit | `9ed653c53e640c84611bdf76818ba7392cece600`; build inputs byte-identical to the approved commit |
| Facade SHA-256 | `9470aa8ab9321d38438c8b10662bb93e622349a28c158a45d786e2af9b9b5873`, reproduced by a fresh pack |
| Native artifacts | the five CI hashes recorded in the [PHASE-16 record](phase-16.md) |
| Published layout | facade with five `optionalDependencies`; platform packages carry only `sendust.node` |
| Publication action | none; `npm publish` appears in the tree only as an explanatory comment |

## 2. Task evidence

| Task | Status | Evidence |
| --- | --- | --- |
| P17-001 | COMPLETED | [RC verification](../qa/rc-verification.md): identity, contents, evidence completeness, critical smokes from the candidate artifacts, and no mismatch |
| P17-002 | COMPLETED | [MVP acceptance matrix](../qa/mvp-acceptance.md): all 17 acceptance items and all 30 constraints PASS with evidence; approved by the project owner on 2026-11-18 |
| P17-003 | COMPLETED | [Publication checklist](../engineering/publication-checklist.md): inputs, pre-flight checks, publish order, post-publication verification, tag, rollback, and the owner's authorization; the executable path is the CI `publish` job with `scripts/publish-release.mjs` |
| P17-004 | BLOCKED | Prerequisites in place: the organization and token exist. Waits on the owner's manual dispatch of the CI `publish` job; the `Rust workspace` flake in section 5 is fixed |
| P17-005 | NOT STARTED | Blocked by P17-004 |

## 3. Acceptance summary

- Two crates, core free of Node, one-way `napi -> core`, native SMTP engine, one
  Tokio runtime, Node-API 8.
- Relay acceptance only; `rejected` on an explicit negative final reply;
  `unknown` after DATA with no retry; per-recipient outcomes preserved.
- `BUSY` with no queue and no dial; explicit bounded close; finite lower-only
  timeouts; copied buffers; opt-in regular-file attachments; hard caps.
- TLS verification always on with no bypass; no public `AbortSignal`.
- No unsupported platform, runtime, or capability is claimed in the package
  metadata or the release notes.

Every claim traces to a named test, CI job, or approved gate record; the rows and
evidence are in the [matrix](../qa/mvp-acceptance.md).

## 4. Publication readiness

| Item | State |
| --- | --- |
| Facade tarball and hash | READY |
| Five platform tarballs | Produced by `scripts/pack-platform.mjs` from the same artifacts; the `Release candidate bundle` job packs and hashes all five |
| Hashes match the approved candidate | READY |
| Secret scan | READY — clean |
| Release notes | READY — install command is updated for the registry at publication |
| Support matrix and licence | READY |
| SBOM | Dependency inventory only; the CycloneDX/SPDX deferral recorded in PHASE-16 stands |
| Approved hash set | `release/0.1.0-rc.1/approved.json`: facade hash, five artifact hashes, and `byteReproducible`; checked by `scripts/publish-release.mjs` with the bundle's `provenance.json` and `SHA256SUMS` |
| Publish automation | The CI `publish` job (manual dispatch with `publish: true`): rebuilds the facade, verifies hashes, publishes the five platform packages then `@sendust/ts` at `next`, and refuses an existing version |
| npm scope | READY — the `sendust` organization exists; `npm org ls sendust` reports `fanvercel - owner` |
| Publish token | READY — the owner reports the `NPM_TOKEN` repository secret is set; the publish job's `npm whoami` step verifies it before publishing |
| Authorization | APPROVED 2026-11-18 |

## 5. Findings

| Severity | Finding | State |
| --- | --- | --- |
| Medium | The publication inputs had no packed platform tarballs: CI uploaded only the raw `.node` files, so a person publishing by hand had to invent the package manifests | FIXED: `scripts/pack-platform.mjs` packs all five from the loader's map at the facade version, with `os`/`cpu`/`libc`, and the `Release candidate bundle` job runs it and records `PLATFORM-PACKAGES.sha256` |
| Low | The phase-17 plan still described the runtime path as `TS -> napi -> core -> lettre` and called a public `AbortSignal` optional | FIXED: the plan now names the native engine (ADR-0017) and records that C-17 prohibits a public `AbortSignal` |
| Medium | `repeated_failed_connections_do_not_leak_descriptors` failed on CI with `descriptor leak across refused connections: 9 -> 18`: `#[tokio::test]` built each probe's runtime before its body took the serial mutex, so the other three probes' idle runtimes (three descriptors each) landed in the process-wide count after the baseline snapshot | FIXED: the probes build their runtime behind a blocking guard, so a probe that has not started has opened nothing; measured on macOS that an idle runtime holds exactly three descriptors and drops back on release |
| Medium | Publication would have run from a developer machine, against the release strategy, and that machine holds only the macOS arm64 artifact | FIXED: a CI `publish` job publishes the six packages from the same run's verified bundle; `--dry-run` printing the plan was exercised locally and correctly warns that the local binary is not the approved CI one |
| Low | The staged publish manifest keeps the repository's `scripts` and `devDependencies` | Deferred: consumers are unaffected and stripping them would change the approved facade hash |
| Medium | The first `publish` dispatch failed with `@sendust/ts-windows-x64 packs a binary that is not the approved windows-x64 artifact`: the gate required cross-run byte equality of a Windows DLL that is not byte-reproducible. Two runs at the same commit produced `85bfa57f…` and `fe50601e…`, the approval run `afcd4882…`, while the other four targets reproduced exactly | FIXED: `scripts/publish-release.mjs` now requires the approved commit from `provenance.json`, the bundle's own `SHA256SUMS` per target, the facade hash, and byte equality only for the targets `approved.json` marks `byteReproducible`; `reproducible-build.md` records the evidence |

No high-severity finding is open.

## 6. Decision

**P17-002 — APPROVED by the project owner on 2026-11-18.** The MVP acceptance
matrix is accepted as the release baseline: the candidate is `0.1.0-rc.1` at
`a8a71e7`, every acceptance item and constraint passes with the recorded
evidence, and no blocking defect or contract deviation is open.

**P17-003 — APPROVED by the project owner on 2026-11-18.** The
[publication checklist](../engineering/publication-checklist.md) is signed for
the six `@sendust` packages at `0.1.0-rc.1` with the `next` dist-tag, from CI,
with the facade and the byte-reproducible targets matching their approved hashes
and Windows identified by the approved commit.

**P17-004 — ready, awaiting the dispatch.** The `sendust` organization exists
with `fanvercel` as owner, and the owner reports the `NPM_TOKEN` repository
secret is set. The descriptor-probe flake that failed the `Rust workspace` job is
fixed, as is the Windows hash the gate over-claimed, so the remaining action is
the owner's: run the `CI` workflow manually with `publish` checked. Nothing is
published until that job completes.

Nothing has been published. A checksum mismatch, a failed consumer check before
publication, or any publication taken outside this authorization reopens the
gate.

## 7. Deferred and non-goals

- Deferred: signing or notarizing binaries, a CycloneDX/SPDX SBOM, and moving a
  dist-tag after publication.
- Deferred to a later version: stripping development metadata from the publish
  manifest.
- Non-goals: publishing an unapproved artifact, bumping a version to cover a
  publication gap, or any post-publication patch that is not a new version.
