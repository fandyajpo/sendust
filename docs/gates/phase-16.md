# QG-16 — Release Engineering Phase Record

**Decision:** AWAITING APPROVAL — project owner review
**Phase:** P16 — Release engineering
**Gate:** PHASE-16
**Prepared:** 2026-11-18
**Candidate:** `0.1.0-rc.1` at commit `023f6fc3b60590d177dd3ea834f0f6737dbf5d54`
**Branch:** `main`

This record is the release-candidate handoff for PHASE-16. No publish command has
run; publication is P17 and needs its own approval. The record was prepared by an
agent, so the decision in section 6 is the required human review step.

## 1. Candidate identity

| Item | Value |
| --- | --- |
| Version | `0.1.0-rc.1` (root and facade manifests, Cargo workspace, facade constant, lockfile, changelog — all agree) |
| Commit | `023f6fc3b60590d177dd3ea834f0f6737dbf5d54` |
| Working tree at assembly | clean |
| Rust toolchain | 1.100.0 (pinned by `rust-toolchain.toml`) |
| Node.js / npm | 22.23.5 / 10.9.2 locally; CI pins npm 12.2.2 |
| Facade tarball | `artifacts/publish/sendust-ts-0.1.0-rc.1.tgz` |
| Facade SHA-256 | `9470aa8ab9321d38438c8b10662bb93e622349a28c158a45d786e2af9b9b5873` |

### Artifact hashes

| Target | SHA-256 | Source |
| --- | --- | --- |
| `macos-arm64` | `a39b370a6c28ef96a70c37bd133d815e6801629d858a079234fd49909221eb86` | Local clean build of the candidate commit; hash recorded in `artifacts/release/0.1.0-rc.1/SHA256SUMS` |
| `linux-x64-gnu` | produced by the `Native artifact (linux-x64)` job on this commit | CI artifact, hashed by its `assemble-rc` step |
| `linux-arm64-gnu` | produced by the `Native artifact (linux-arm64)` job on this commit | Same |
| `macos-x64` | produced by the Native artifact (`macos-15-intel`) job on this commit | Same |
| `windows-x64` | produced by the `Native artifact (windows-x64)` job on this commit | Same |

Native artifacts are per-runner builds, so each runner's hash is authoritative for
its own target; P17 verifies the set against the CI run for this commit before
publishing anything. The facade tarball is target-independent and its hash above
is the one P17 must match.

## 2. Task evidence

| Task | Status | Evidence |
| --- | --- | --- |
| P16-001 | COMPLETED | [Versioning policy](../engineering/versioning-policy.md), `CHANGELOG.md`, and `scripts/check-versions.mjs`; `PASS: version 0.1.0-rc.1 agrees across the manifests, the lockfile, the facade, and the changelog`. |
| P16-002 | COMPLETED | [Reproducible build and provenance](../engineering/reproducible-build.md) and `scripts/assemble-rc.mjs`; `artifacts/release/0.1.0-rc.1/` holds `provenance.json` (98 Rust crates, 101 npm packages), `dependencies.json`, and `SHA256SUMS`. |
| P16-003 | COMPLETED | [0.1.0-rc.1 release notes](../releases/0.1.0-rc.1.md): support matrix, install, semantics, limits, security, limitations, no migration. |
| P16-004 | COMPLETED | [Release checklist and rollback plan](../engineering/release-checklist.md): 14 checks with owner, evidence, and pass condition; the assembly/publication boundary; five rollback scenarios. |
| P16-005 | IN REVIEW | This record. |

## 3. Checklist status

| # | Check | Status |
| --- | --- | --- |
| 1 | Version agreement | PASS — check-versions on the candidate commit |
| 2 | Clean build | PASS — Rust workspace job green on `023f6fc`; local `build:native` and the 61-test suite pass |
| 3 | TypeScript contract | PASS — Node 22/24/26 jobs |
| 4 | Runtime smokes and behaviour | PASS — binding and artifact jobs (Node, Bun, Deno), doc sample |
| 5 | Security gate | PASS — PHASE-11 approved; secret scan clean; redaction tests in the suite |
| 6 | Performance report | PASS — PHASE-13 approved budgets |
| 7 | Distribution matrix | PASS — PHASE-14 approved, five targets |
| 8 | Documentation accuracy | PASS — PHASE-15 approved; `check-docs`, `check-doc-claims`, doc sample |
| 9 | Artifact checksums | PASS — `SHA256SUMS` for the local artifact; per-target hashes from each CI artifact job |
| 10 | Dependency inventory | PASS — `dependencies.json` (98 Rust crates, 101 npm packages) |
| 11 | Licence | PASS — `MIT OR Apache-2.0` in both manifests and the facade tarball |
| 12 | Publish manifest | PASS — declares the five platform packages at `0.1.0-rc.1`, `private: false`, no bundled addon |
| 13 | No publication action | PASS — no `npm publish` in `scripts/` or `.github/` |
| 14 | Clean tree | PASS — assembly ran on a clean commit |

## 4. Acceptance criteria

| Phase-16 criterion | Result | Evidence |
| --- | --- | --- |
| Versioning, changelog, and compatibility policy are complete and honest | PASS | Versioning policy, changelog categories, compatibility table; enforced by the version check |
| Build inputs and artifacts are reproducible and provenance-tracked | PASS | Pinned inputs; byte-identical clean rebuild documented; provenance with commit, toolchain, hashes, inventory |
| Release notes contain accurate semantics and explicit limitations | PASS | Notes match PHASE-15 guides; no delivery guarantee, retry, pooling, SMTPUTF8, URL attachment, or unsupported platform |
| Checklist and rollback plan cover all release risks | PASS | 14 checks and five rollback scenarios |
| Candidate is assembled with hashes and no publication | PASS | Candidate identity above; no publish command executed |

## 5. Findings

| Severity | Finding | State |
| --- | --- | --- |
| High | Cutting the candidate version was impossible at first: `sendust-napi` pinned `sendust-core = "=0.1.0"`, so the workspace bump broke the build | FIXED: the adapter uses a path-only dependency; `check-versions` now rejects such a pin |
| Low | A stale artifact from the previous version was picked up by the assembler during the failed build | FIXED: the stale file was removed and the artifact rebuilt; the gate records the candidate hash |

No high-severity finding is open.

## 6. Decision

**AWAITING APPROVAL.** The project owner must confirm the release-candidate
handoff:

1. The candidate identity: `0.1.0-rc.1` at commit `023f6fc`, with the facade
   hash and the per-target artifact hashes as recorded.
2. The checklist in section 3 is the evidence P17 must re-verify, not re-derive.
3. No publication has happened and none happens until P17 approves it, platform
   packages first.

Approval makes P16-001 to P16-005 COMPLETED and PHASE-16 APPROVED, which makes
P17 eligible. A failed check on the candidate commit, a checksum mismatch, or a
publication action reopens this gate.

## 7. Deferred and non-goals

- Deferred: downloading and re-hashing the five CI artifacts as one bundle,
  signing or notarizing binaries, a CycloneDX/SPDX SBOM, and the tag itself
  (created only when publication is approved).
- Non-goals: publishing, moving a tag, replacing a tarball, or claiming delivery.
