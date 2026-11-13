# Milestone Registry

Milestones are evidence checkpoints, not calendar dates. A milestone is reached only when every mapped gate task is `COMPLETED` and its evidence is accepted. M0 is reached with QG-00, QG-01, and QG-02 records; all implementation milestones remain pending. Effort is relative (S/M/L/XL) and never a delivery estimate.

## Milestone summary

| ID | Milestone | Gate tasks | Depends on | Effort |
| --- | --- | --- | --- | --- |
| M0 | Architecture Approved | P02-005 | P00-005, P01-005 | M |
| M1 | Native Binding Proof of Concept | P05-005 | P03-005, P04-005 | L |
| M2 | SMTP Foundation | P06-005 | P01-005, P04-005 | XL |
| M3 | Email Composition | P07-005, P08-005 | P04-005, P05-005 | L |
| M4 | Functional MVP | P09-005 | P05-005, P06-005, P08-005 | L |
| M5 | Cross-Platform Package | P14-005 | P12-005, P13-005, P05-005, P03-005 | L |
| M6 | Release Candidate | P16-005 | P14-005, P12-005, P13-005, P15-005 | L |
| M7 | MVP Release | P17-005 | P16-005 | M |

## M0 — Architecture Approved

**REACHED — 2026-11-13**

**Evidence:** [QG-00](../gates/phase-00-discovery.md), [QG-01](../gates/phase-01-research.md), [QG-02](../gates/phase-02-architecture-approval.md), [approval matrix](../architecture/approval-matrix.md).

**Purpose:** Freeze the problem, constraints, dependency decisions, and architecture so implementation cannot drift.

**Required gates:**
- P00-005: discovery brief, constraint ledger, provenance, threat boundary, ADR seeds.
- P01-005: lettre API decision, TLS feature decision, recipient policy decision, Node lifecycle decision, no-custom-SMTP confirmation.
- P02-005: two-crate boundary, status/result/error contract, lifecycle and timeout model, ownership and limits, ADR freeze, traceability matrix.

**Exit criteria:**
- Every constraint C-01 through C-30 maps to an enforcement task.
- Every mandatory ADR is `ACCEPTED` or explicitly `BLOCKED` with an architecture-time deadline.
- No implementation is required to understand the public contract.

**Residual implementation risks:** Cargo feature compilation, Node runtime compatibility, SMTP/TLS fake-relay tests. These do not block the design baseline but remain hard implementation gates before dependent behavior is claimed.

## M1 — Native Binding Proof of Concept

**Purpose:** Prove the core can be called from Node.js with correct ownership, async behavior, and lifecycle.

**Required gates:**
- P03-005: workspace builds, lints, formats, unit tests run; fake SMTP/TLS seams exist.
- P04-005: core errors, validation, message model, state machine, and lettre orchestration pass core tests without Node.
- P05-005: Node-API 8 binding compiles, copies buffers, maps errors, handles async send/close, and passes lifecycle tests.

**Exit criteria:**
- `cargo test -p sendust-core` passes with no Node linkage.
- A Node test can create a transport, call an async function, receive a stable error, and close.
- Buffer mutation/detachment does not affect sent bytes.
- Forced GC does not close the transport.
- Candidate Node 22/24/26 results are recorded; unsupported candidates are labeled.

**Risks:** borrowed memory, runtime mismatch, panic across FFI.  
**Mitigation:** copy-before-async rule, napi-managed Tokio only, `Result` at boundaries, forced-GC tests.

## M2 — SMTP Foundation

**Purpose:** Deliver a trustworthy SMTP transaction with mandatory TLS, authentication, honest outcomes, and no retry.

**Required gates:**
- P06-005: full SMTP fault matrix, exact command traces, recipient policy implementation, TLS certificate failures, timeout boundaries.

**Exit criteria:**
- Success returns `accepted` only after a positive final DATA response.
- Pre-DATA rejection returns `rejected`; post-DATA ambiguity returns `unknown`.
- No retry and no per-recipient independent messages.
- Certificate and hostname verification cannot be disabled.
- Auth and TLS failures map to stable, redacted errors.
- Timeouts report category and elapsed budget; stage only if proven observable.

**Risks:** hidden recipient status, TLS feature drift, timeout misclassification.  
**Mitigation:** lower-level API inspection, pinned features, scripted fake relay.

## M3 — Email Composition

**Purpose:** Compose standards-compliant MIME messages with text, HTML, headers, and safe attachments.

**Required gates:**
- P07-005: MIME conformance, header encoding, alternative/mixed structure, wire-size parity.
- P08-005: buffer and opt-in file attachments, safe regular-file reads, caps, filename/content-type encoding.

**Exit criteria:**
- Independent parser accepts every emitted message.
- BCC never appears in headers.
- Attachment limits pass at boundary and fail at boundary + 1.
- Symlink, FIFO, device, directory, and missing paths are rejected.
- No URL/stream/directory attachment form exists.

**Risks:** encoding edge cases, TOCTOU, memory duplication.  
**Mitigation:** parser-based conformance, open-then-fstat, cap-probe reads, measured memory.

## M4 — Functional MVP

**Purpose:** Provide a coherent public TypeScript API over the proven engine.

**Required gates:**
- P09-005: public contract, type tests, examples, lifecycle, errors, redaction, BUSY, close.

**Exit criteria:**
- Public API exposes only MVP features.
- `send` returns structured outcomes and never claims delivery.
- `BUSY`, `CLOSED`, timeout, TLS, auth, validation, limit, and internal errors are stable and catchable.
- Close is idempotent and bounded.
- No pooling, retry, SMTPUTF8, URL attachments, or required AbortSignal.

**Risks:** semantic drift in the wrapper, accidental convenience features.  
**Mitigation:** contract gate, type tests, API surface audit.

## M5 — Cross-Platform Package

**Purpose:** Prove the package installs and loads from npm-style artifacts on the declared matrix.

**Required gates:**
- P10-005: reliability fault injection and cleanup evidence.
- P11-005: security regression and remediation sign-off.
- P12-005: requirements-to-test traceability and coverage evidence.
- P13-005: performance budgets and regression report.
- P14-005: clean installs, native loading, runtime behavior, and support declaration.

**Exit criteria:**
- Every declared target installs and loads the correct binary.
- Missing/wrong-architecture binaries fail clearly.
- Candidate Node versions are validated or explicitly unsupported.
- No install-time network or secret dependency.
- Support matrix matches CI.

**Risks:** platform build gaps, wrong binary selection, untested Node versions.  
**Mitigation:** early smoke (P14-001), explicit matrix, per-target validation, no untested claims.

## M6 — Release Candidate

**Purpose:** Assemble a reproducible, auditable release candidate without publishing.

**Required gates:**
- P15-005: documentation accuracy gate.
- P16-005: versioning, reproducible build, provenance, notes, checklist, rollback plan, RC handoff.

**Exit criteria:**
- Candidate version, commit, and artifact hashes recorded.
- Two clean builds produce identical hashes or documented nondeterminism.
- Release notes match behavior and limitations.
- Checklist covers security, QA, performance, distribution, docs, and package integrity.
- No publish command has run.

**Risks:** nondeterministic builds, stale docs, accidental publication.  
**Mitigation:** reproducibility test, docs gate before RC, publish credentials absent from candidate jobs.

## M7 — MVP Release

**Purpose:** Authorize and publish the MVP, then verify the published package.

**Required gates:**
- P17-001: RC identity and evidence completeness.
- P17-002: final MVP acceptance against constraints and feature matrix.
- P17-003: publication checklist and authorization.
- P17-004: publish and post-publication verification.
- P17-005: approval record, closure, rollback readiness.

**Exit criteria:**
- Published root package and every platform package share the exact version.
- Clean install from the registry works on representative targets.
- Provenance and release assets are present.
- No unsupported claim is published.
- Rollback/deprecation procedure is documented and owned.

**Risks:** partial publication, wrong optional dependency version, missing artifact.  
**Mitigation:** artifact completeness gate, version synchronization, post-publication verification, recovery procedure.

## Milestone-to-phase map

| Milestone | Phases involved |
| --- | --- |
| M0 | P00, P01, P02 |
| M1 | P03, P04, P05 |
| M2 | P06 |
| M3 | P07, P08 |
| M4 | P09 |
| M5 | P10, P11, P12, P13, P14 |
| M6 | P15, P16 |
| M7 | P17 |
| Post-MVP | P18 |
