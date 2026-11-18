# QG-17 — RC and MVP approval record

**Decision:** P17-002 and P17-003 APPROVED by the project owner, 2026-11-18;
P17-004 published to npm on 2026-11-18 and verified from the registry; P17-005
closure prepared, PHASE-17 approval pending the project owner
**Phase:** P17 — RC and MVP approval/publication
**Gate:** PHASE-17
**Prepared:** 2026-11-18
**Candidate:** `0.1.0-rc.1` at commit `a8a71e7b613e0c56bbd48655ba5a2aedaa55afa5`
**Branch:** `main`

This record is the approval package for PHASE-17. The project owner accepted the
MVP (P17-002) and authorized publication (P17-003); the CI `publish` job then
published the six `@sendust` packages at `0.1.0-rc.1` on 2026-11-18 (P17-004),
and the published artifact was verified from the registry rather than the
workspace (section 4). The phase decision itself is the project owner's,
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
| Publication action | CI run 44230251909, `publish` job 134555264572 on `37f4146`; six packages at `0.1.0-rc.1`, dist-tag `next` |

## 2. Task evidence

| Task | Status | Evidence |
| --- | --- | --- |
| P17-001 | COMPLETED | [RC verification](../qa/rc-verification.md): identity, contents, evidence completeness, critical smokes from the candidate artifacts, and no mismatch |
| P17-002 | COMPLETED | [MVP acceptance matrix](../qa/mvp-acceptance.md): all 17 acceptance items and all 30 constraints PASS with evidence; approved by the project owner on 2026-11-18 |
| P17-003 | COMPLETED | [Publication checklist](../engineering/publication-checklist.md): inputs, pre-flight checks, publish order, post-publication verification, tag, rollback, and the owner's authorization; the executable path is the CI `publish` job with `scripts/publish-release.mjs` |
| P17-004 | COMPLETED | Published from CI on 2026-11-18 and verified from the registry (section 4): six packages at `0.1.0-rc.1`, facade hash matches the approval, clean consumer install on macOS arm64, and 16/16 behaviour checks on Node.js, Bun, and Deno |
| P17-005 | IN PROGRESS | This record: the publication, verification, limitation, and rollback closure; the phase decision in section 6 awaits the project owner |

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

## 4. Publication and verification

The `publish` job ran from commit `37f4146`, which the publisher accepted as a
build of the approved `a8a71e7` because `git diff` found no change to any build
input. The commits after approval touched documentation, tests, the leak probe,
the publish tooling, and release metadata only. Nothing published was replaced:
the version did not exist before the run.

| Package | Role | State |
| --- | --- | --- |
| `@sendust/ts` | facade | published at `0.1.0-rc.1` |
| `@sendust/ts-linux-x64-gnu` | platform | published at `0.1.0-rc.1` |
| `@sendust/ts-linux-arm64-gnu` | platform | published at `0.1.0-rc.1` |
| `@sendust/ts-macos-x64` | platform | published at `0.1.0-rc.1` |
| `@sendust/ts-macos-arm64` | platform | published at `0.1.0-rc.1` |
| `@sendust/ts-windows-x64` | platform | published at `0.1.0-rc.1` |

Every check below used the registry, not the workspace:

| Check | Command | Result |
| --- | --- | --- |
| Facade tarball hash | `npm pack @sendust/ts@0.1.0-rc.1` | `9470aa8ab9321d38438c8b10662bb93e622349a28c158a45d786e2af9b9b5873`, the approved value |
| Optional dependencies | `npm view @sendust/ts@0.1.0-rc.1 optionalDependencies` | the five platform packages, all pinned to `0.1.0-rc.1` |
| Platform metadata | `npm view @sendust/ts-<target>@0.1.0-rc.1 os cpu libc` | `linux`/`darwin`/`win32`, `x64`/`arm64`, `glibc` on Linux only |
| Clean consumer install | `npm i @sendust/ts@0.1.0-rc.1` in an empty project | `added 2 packages`: the facade and only `@sendust/ts-macos-arm64` |
| Behaviour contract, Node.js 22.23.5 | `node tests/runtime/behaviour.mjs` | 16/16 checks |
| Behaviour contract, Bun 1.3.14 | `bun tests/runtime/behaviour.mjs` | 16/16 checks |
| Behaviour contract, Deno 2.9.8 | `deno run --allow-ffi --allow-read --allow-net tests/runtime/behaviour.mjs` | 16/16 checks |
| Dist-tags | `npm view @sendust/ts dist-tags` | `next` is `0.1.0-rc.1`; npm also assigned `latest` to the first publish (finding in section 5) |

Pre-flight state at publication: the facade and five platform tarballs came from
the `Release candidate bundle` job of the same run, `scripts/publish-release.mjs`
matched the facade hash, read `sendust.node` back out of every tarball against the
bundle's `SHA256SUMS`, and required byte equality for the four `byteReproducible`
targets (`linux-x64-gnu`, `linux-arm64-gnu`, `macos-arm64`, `macos-x64`). The
Windows artifact of that run was `94983d31…`, the fourth distinct value observed
for that target and the reason Windows is held to the approved build inputs.

The local Bun is 1.3.14, not the pinned 1.4.4; the CI behaviour suite runs 1.4.4
per target, which remains the support claim.

## 5. Findings

| Severity | Finding | State |
| --- | --- | --- |
| Medium | The publication inputs had no packed platform tarballs: CI uploaded only the raw `.node` files, so a person publishing by hand had to invent the package manifests | FIXED: `scripts/pack-platform.mjs` packs all five from the loader's map at the facade version, with `os`/`cpu`/`libc`, and the `Release candidate bundle` job runs it and records `PLATFORM-PACKAGES.sha256` |
| Low | The phase-17 plan still described the runtime path as `TS -> napi -> core -> lettre` and called a public `AbortSignal` optional | FIXED: the plan now names the native engine (ADR-0017) and records that C-17 prohibits a public `AbortSignal` |
| Medium | `repeated_failed_connections_do_not_leak_descriptors` failed on CI with `descriptor leak across refused connections: 9 -> 18`: `#[tokio::test]` built each probe's runtime before its body took the serial mutex, so the other three probes' idle runtimes (three descriptors each) landed in the process-wide count after the baseline snapshot | FIXED: the probes build their runtime behind a blocking guard, so a probe that has not started has opened nothing; measured on macOS that an idle runtime holds exactly three descriptors and drops back on release |
| Medium | Publication would have run from a developer machine, against the release strategy, and that machine holds only the macOS arm64 artifact | FIXED: a CI `publish` job publishes the six packages from the same run's verified bundle; `--dry-run` printing the plan was exercised locally and correctly warns that the local binary is not the approved CI one |
| Low | The staged publish manifest keeps the repository's `scripts` and `devDependencies` | Deferred: consumers are unaffected and stripping them would change the approved facade hash |
| Low | npm assigned `latest` to `0.1.0-rc.1` alongside `next` on the first publish, and the registry refuses to remove it (`DELETE …/dist-tags/latest` returns 400). A pre-release therefore sits on `latest` until the first stable release moves it | Documented: the release notes name `@sendust/ts@next` and the exact version, and the [versioning policy](../engineering/versioning-policy.md) moves `latest` at the first stable release |
| Medium | The first `publish` dispatch failed with `@sendust/ts-windows-x64 packs a binary that is not the approved windows-x64 artifact`: the gate required cross-run byte equality of a Windows DLL that is not byte-reproducible. Two runs at the same commit produced `85bfa57f…` and `fe50601e…`, the approval run `afcd4882…`, while the other four targets reproduced exactly | FIXED: `scripts/publish-release.mjs` requires the approved commit or a later one whose build inputs are byte-identical, reads the addon back out of every tarball and ties it to the bundle's `SHA256SUMS`, and compares bytes only for the targets `approved.json` marks `byteReproducible`; `reproducible-build.md` records the evidence |

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
and Windows identified by the approved build inputs and the bundle's checksums.

**P17-004 — COMPLETED on 2026-11-18.** The six `@sendust` packages are published
at `0.1.0-rc.1` from the CI `publish` job, the facade tarball on the registry
reproduces the approved `9470aa8a…`, and the clean-consumer behaviour contract
passes on Node.js, Bun, and Deno (section 4).

**P17-005 — closure prepared, awaiting PHASE-17 approval.** This record carries
the publication evidence, the two remaining findings, the known limitations
(npm's `latest` assignment, the pre-existing deferred items), and the rollback
path in the [publication checklist](../engineering/publication-checklist.md).
The release is published; the annotated tag `v0.1.0-rc.1` on
`a8a71e7b613e0c56bbd48655ba5a2aedaa55afa5` is the project owner's to create,
since no tag tool exists in this environment.

A checksum mismatch in the facade or a byte-reproducible target, a failed
consumer check, or any publication taken outside this authorization reopens the
gate; a post-publication defect follows the rollback plan rather than an ad hoc
patch.

## 7. Deferred and non-goals

- Deferred: signing or notarizing binaries, a CycloneDX/SPDX SBOM, and moving a
  dist-tag after publication. `latest` cannot be removed from the registry; the
  first stable release moves it.
- Deferred to a later version: stripping development metadata from the publish
  manifest.
- Non-goals: publishing an unapproved artifact, bumping a version to cover a
  publication gap, or any post-publication patch that is not a new version.
