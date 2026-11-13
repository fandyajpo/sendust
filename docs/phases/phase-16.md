# Phase 16 — Release engineering

**Phase status:** NOT STARTED  
**Phase dependency:** Phase 14, Phase 12, Phase 15, Phase 13  
**Companion references:** [release strategy](../engineering/release-strategy.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md), [performance strategy](../engineering/performance-strategy.md), [system design](../architecture/system-design.md)

## Objective

Prepare a reproducible, auditable release candidate process without publishing it. Release engineering must assemble versioning, build provenance, artifact checksums, changelog, and release gates from validated QA, performance, distribution, and documentation evidence.

## Scope

In scope: release checklist, versioning policy, reproducible build inputs, artifact signing/checksums where applicable, changelog, release notes draft, release-candidate assembly, and rollback plan. Out of scope: actual publication, RC approval, and post-MVP changes. No package is published in this phase.

## Tasks

### P16-001 — Define versioning, tagging, and changelog policy

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P14-005, P15-005
- **Description:** Specify semantic versioning, pre-release identifiers, tag format, version source of truth, and changelog categories. Define compatibility promises for Node candidate versions, platforms, and the public API. State that delivery, retry, pooling, SMTPUTF8, and URL attachments are not provided. Include how breaking changes to status semantics are handled.
- **Evidence criteria:** Policy document covers major/minor/patch/pre-release rules, tag naming, and changelog format. Compatibility statement matches Phase 14 support matrix and Phase 15 docs. No promise exceeds tested behavior.

### P16-002 — Assemble reproducible build and artifact provenance

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P16-001
- **Description:** Define deterministic build inputs: pinned Rust toolchain, lockfile, napi/Node-API 8 settings, TypeScript build, target matrix, and environment controls. Specify artifact checksums, SBOM or dependency inventory, and provenance metadata. Ensure builds do not fetch unverified code or require secrets. Record how rustls `ring`/`webpki-roots` features are fixed.
- **Evidence criteria:** Build recipe reproduces identical artifact checksums in a clean environment, or documents unavoidable nondeterminism. Provenance includes source commit, toolchain, dependency inventory, and target. No secret or external relay is used. Feature set matches P01-002.

### P16-003 — Draft release notes and migration/limitations section

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P16-001, P15-005
- **Description:** Draft release notes from the changelog and documentation: supported platforms and Node candidates, installation, relay-accepted semantics, unknown-after-DATA, BUSY, close, caps, attachment opt-in, TLS requirements, and explicit non-goals. Include known limitations and any migration notes for API/status changes.
- **Evidence criteria:** Notes contain no delivery guarantee, no retry/pooling claim, and no unsupported platform. Every limitation links to documentation. Status semantics match P02/P09/P15 exactly. Draft is reviewed by QA and security owners.

### P16-004 — Create the release checklist and rollback plan

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P16-002, P16-003, P12-005, P13-005
- **Description:** Define pre-release checks: clean build, full test matrix, security gate, performance report, distribution matrix, documentation accuracy, artifact checksums, license/SBOM, and secret scan. Define rollback/unpublish criteria, yanked-version process, consumer communication, and artifact quarantine. Distinguish candidate assembly from publication approval.
- **Evidence criteria:** Checklist has owner, evidence link, and pass condition for every item. Rollback plan covers bad binary, contract bug, security issue, and install breakage. No checklist item relies on unpublished evidence. Publication remains explicitly out of scope.

### P16-005 — Release-engineering gate and RC handoff

- **Size:** S
- **Status:** NOT STARTED
- **Prerequisites:** P16-001, P16-002, P16-003, P16-004
- **Description:** Assemble the release candidate package and hand off to Phase 17. Verify all prerequisite gates are approved, artifacts are reproducible, notes and checklist are complete, and no publication action has occurred. Record the candidate version and artifact hashes.
- **Evidence criteria:** Gate record includes candidate version, commit, artifact hashes, checklist status, and links to QA/performance/distribution/docs evidence. No publish command is run. Phase 17 receives an explicit approve/reject package.

## Technical implementation details

Release engineering preserves the frozen contract. The build produces the napi binding for Node-API 8 from two crates: `crates/sendust-core/src` without Node linkage and `crates/sendust-napi/src` as adapter, with `ts/src` as the wrapper and `lettre 0.11.23` as the SMTP implementation. TLS uses rustls `ring` and `webpki-roots`; verification cannot be disabled. Candidate Node 22/24/26 support is declared only as validated by Phase 14. Success is relay acceptance; post-DATA ambiguity is `unknown` and not retried. One active send returns `BUSY`; no queue, pooling, or automatic retry. Close is explicit and bounded by 125s, never GC-dependent. Timeouts are connect 10s, operation 30s, total 120s, close 125s. Caps are 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw, 30 MiB wire. Attachments require opt-in and regular files only; no URLs. No secrets in artifacts, notes, or build logs.

## Dependencies

Phase 16 requires Phase 14, Phase 12, Phase 15, and Phase 13. P16-001 depends on P14-005 and P15-005; P16-002 depends on P16-001; P16-003 adds P15-005; P16-004 adds P12-005 and P13-005; P16-005 depends on all. Phase 17 requires P16-005.

## Risks and edge cases

Nondeterministic native builds, toolchain drift, missing artifact provenance, stale release notes, unverified support claims, secrets in build logs, and accidental publication during candidate assembly are key risks. A checksum mismatch must block the candidate. Rollback must account for already-installed consumers and yanked native binaries.

## Testing requirements

Validate reproducibility by building twice in clean environments and comparing hashes. Run the full QA matrix, security regression, performance report, distribution clean-install matrix, and documentation link/sample checks as checklist evidence. Verify artifact contents exclude tests, source maps, secrets, and unsupported platform binaries. Verify no publish command executes. Add a negative check for missing SBOM, missing checksum, and failed secret scan.

## Acceptance criteria

- Versioning, changelog, and compatibility policy are complete and honest.
- Build inputs and artifacts are reproducible and provenance-tracked.
- Release notes contain accurate semantics and explicit limitations.
- Checklist and rollback plan cover all release risks.
- Candidate is assembled with hashes and no publication.

## Deliverables

Versioning/changelog policy, reproducible build recipe, provenance/SBOM, release notes draft, release checklist, rollback plan, and RC handoff package.

## Completion gate

`PHASE-16` is approved only when P16-005 proves a reproducible, fully evidenced release candidate with no publication action and no unsupported claim. Phase 17 may begin only after approval.

## Status

NOT STARTED.
