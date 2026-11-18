# QG-17 — RC and MVP approval record

**Decision:** PENDING — RC verified and acceptance matrixed; publication not
authorized
**Phase:** P17 — RC and MVP approval/publication
**Gate:** PHASE-17
**Prepared:** 2026-11-18
**Candidate:** `0.1.0-rc.1` at commit `a8a71e7b613e0c56bbd48655ba5a2aedaa55afa5`
**Branch:** `main`

This record is the approval package for PHASE-17. No publish command has run and
no artifact has been sent to a registry. The two remaining decisions — accepting
the MVP against the authoritative constraints (P17-002) and authorizing
publication (P17-003) — are the project owner's, and section 6 is empty until
they are recorded.

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
| P17-002 | IN REVIEW | [MVP acceptance matrix](../qa/mvp-acceptance.md): all 17 acceptance items and all 30 constraints PASS with evidence; the approve/reject decision is section 6 |
| P17-003 | IN REVIEW | [Publication checklist](../engineering/publication-checklist.md): inputs, pre-flight checks, publish order, post-publication verification, tag, and rollback; the authorization table is PENDING |
| P17-004 | NOT STARTED | Blocked by P17-003 authorization |
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
| npm scope, 2FA, token | Owner action — not part of the repository |
| Authorization | **PENDING** |

## 5. Findings

| Severity | Finding | State |
| --- | --- | --- |
| Medium | The publication inputs had no packed platform tarballs: CI uploaded only the raw `.node` files, so a person publishing by hand had to invent the package manifests | FIXED: `scripts/pack-platform.mjs` packs all five from the loader's map at the facade version, with `os`/`cpu`/`libc`, and the `Release candidate bundle` job runs it and records `PLATFORM-PACKAGES.sha256` |
| Low | The phase-17 plan still described the runtime path as `TS -> napi -> core -> lettre` and called a public `AbortSignal` optional | FIXED: the plan now names the native engine (ADR-0017) and records that C-17 prohibits a public `AbortSignal` |
| Low | The staged publish manifest keeps the repository's `scripts` and `devDependencies` | Deferred: consumers are unaffected and stripping them would change the approved facade hash |

No high-severity finding is open.

## 6. Decision

**PENDING.** The project owner must record two things, in this order:

1. **MVP acceptance (P17-002):** approve or reject the candidate against the
   [acceptance matrix](../qa/mvp-acceptance.md). A rejection names the blocking
   task and no artifact is published.
2. **Publication authorization (P17-003):** sign the authorization table in the
   [publication checklist](../engineering/publication-checklist.md), naming the
   six packages, the candidate version and hashes, and the `next` dist-tag.

Until both are recorded, P17-004 does not run and nothing is published. A checksum
mismatch, a failed consumer check before publication, or any publication action
taken without the authorization reopens this gate.

## 7. Deferred and non-goals

- Deferred: signing or notarizing binaries, a CycloneDX/SPDX SBOM, and moving a
  dist-tag after publication.
- Deferred to a later version: stripping development metadata from the publish
  manifest.
- Non-goals: publishing an unapproved artifact, bumping a version to cover a
  publication gap, or any post-publication patch that is not a new version.
