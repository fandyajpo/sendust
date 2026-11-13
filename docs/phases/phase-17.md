# Phase 17 — RC and MVP approval/publication

**Phase status:** NOT STARTED  
**Phase dependency:** Phase 16  
**Companion references:** [release strategy](../engineering/release-strategy.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md), [system design](../architecture/system-design.md)

## Objective

Evaluate the release candidate against every MVP gate, obtain explicit approval or rejection, and only then publish the approved artifact. Publication must be a controlled, auditable action with rollback readiness and honest MVP claims.

## Scope

In scope: RC verification, approval review, publication checklist, publication execution, post-publication smoke, and rollback decision. Out of scope: feature development, post-MVP roadmap work, and changing frozen semantics. No publication occurs before approval.

## Tasks

### P17-001 — Verify RC identity, artifacts, and evidence completeness

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P16-005
- **Description:** Confirm candidate version, commit, artifact hashes, and provenance match Phase 16. Verify all prerequisite gates are approved, checklist evidence is present, and no artifact changed after QA/performance/distribution validation. Re-run critical smoke tests from the exact candidate artifacts.
- **Evidence criteria:** Identity record matches hashes and commit. Critical smoke covers accepted send, post-DATA unknown, BUSY, close, TLS verification failure, attachment opt-in, and cap rejection. Any mismatch blocks approval. No unapproved artifact is used.

### P17-002 — Run final MVP acceptance review against authoritative constraints

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P17-001
- **Description:** Review the candidate against every authoritative constraint: two crates with core free of Node, `TS -> napi -> core -> lettre`, `lettre 0.11.23`, no custom SMTP unless research justified it, no pooling, no automatic retries, relay-accepted semantics, unknown after DATA, BUSY with no queue, explicit close, finite timeouts, copied buffers, opt-in regular-file attachments, hard caps, address policy, TLS verification, and optional AbortSignal. Confirm documentation and release notes match.
- **Evidence criteria:** Acceptance matrix marks each constraint pass/fail with evidence. No open high-severity defect or contract deviation. Reviewer records explicit approve or reject. Any rejection names the blocking task and does not publish.

### P17-003 — Execute the publication checklist and release authorization

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P17-002
- **Description:** Prepare publication inputs: package version, registry target, artifact set, checksums, release notes, license/SBOM, support matrix, and rollback contacts. Define who authorizes publication and require recorded approval. Confirm no secrets, no test artifacts, and no unsupported platform binaries are included.
- **Evidence criteria:** Checklist is complete and signed. Authorization record exists before any publish command. Artifact set matches Phase 16 hashes. Secret scan and license/SBOM checks pass. Unsupported platforms and non-goals are absent from publication metadata.

### P17-004 — Publish the approved MVP and run post-publication verification

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P17-003
- **Description:** Publish only the approved artifacts, then verify from a clean consumer environment: install, import, native binary resolution, accepted fake-relay send, BUSY, close, and documentation availability. Do not use production relays or real credentials. Record publication URLs, timestamps, and checksums.
- **Evidence criteria:** Post-publication smoke passes from the registry artifact, not the workspace. Installed artifact hash matches approval. Release notes and support matrix are visible. No post-publication test contacts external SMTP. Any failure triggers the rollback path rather than ad hoc patching.

### P17-005 — MVP approval record, publication closure, and rollback readiness

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P17-001, P17-002, P17-003, P17-004
- **Description:** Close the MVP release with an approval record, publication evidence, known limitations, and rollback status. Confirm the published version is exactly the approved candidate, consumer smoke passed, and support claims are honest. Hand off post-MVP work to Phase 18 without changing MVP semantics.
- **Evidence criteria:** Closure record includes approval signatures, published version/hashes, consumer smoke results, limitations, and rollback readiness. No unresolved blocker remains. Phase 18 receives the approved baseline and explicit non-goals. If publication was rejected, the record states no artifact was published and names the blocking work.

## Technical implementation details

Publication is gated by the architecture approved at P02-005. The artifact must contain exactly the two-crate implementation exposed through napi: `TS -> napi -> core -> lettre`, with `lettre 0.11.23`, Tokio 1 managed by napi, and rustls `ring`/`webpki-roots` with verification always on. No custom SMTP unless Phase 01 explicitly justified it. MVP has no pooling and no automatic retries; success means relay acceptance, not delivery; an explicit negative final DATA reply is `rejected`; a missing/indeterminate final reply after DATA may have reached the relay is `unknown`; neither is retried. One active send returns `BUSY`; no queue. Close is explicit and bounded by 125s, never GC-dependent. Timeouts are connect 10s, operation 30s, total 120s, close 125s, finite and tunable. JS buffers are copied before async. Attachments are opt-in, regular-file only, no URLs, with caps of 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw, and 30 MiB wire. Envelope local parts are ASCII, domains IDNA, display/body Unicode; SMTPUTF8 is post-MVP. AbortSignal is optional and not required. Candidate Node 22/24/26 claims must match validated evidence.

## Dependencies

Phase 17 requires Phase 16. P17-001 depends on P16-005; P17-002 depends on P17-001; P17-003 depends on P17-002; P17-004 depends on P17-003; P17-005 depends on all. No publication occurs before P17-003 authorization. Phase 18 requires P17-005.

## Risks and edge cases

Publishing the wrong artifact, stale release notes, unsupported platform claims, credential leakage in metadata, registry resolution differences, and post-publication failures are key risks. A failed consumer smoke after publication must trigger rollback/yank procedures, not undocumented patches. Approval by an unauthorized person or without evidence must be rejected.

## Testing requirements

From the exact candidate artifact, run accepted send, pre-DATA rejection, mixed RCPT ADR behavior, explicit final DATA `451` rejection and missing-final-reply-after-DATA unknown, BUSY, double close, send-after-close, TLS certificate failure, attachment opt-in disabled/enabled, cap rejection, and buffer mutation smoke. Run from a clean consumer against local fake SMTP/TLS only. Verify published artifact hash, install path, native binary selection, documentation links, and support matrix. Add negative tests for unauthorized publication and hash mismatch.

## Acceptance criteria

- RC identity and evidence are complete and match the approved candidate.
- Every authoritative constraint passes or publication is rejected.
- Publication authorization is recorded before any publish action.
- Post-publication clean-consumer smoke passes with matching hashes.
- Approval record, limitations, and rollback readiness are complete.

## Deliverables

RC verification record, MVP acceptance matrix, publication checklist and authorization, published artifact evidence, post-publication smoke report, MVP approval/closure record, and rollback readiness statement.

## Completion gate

`PHASE-17` is approved only when the authorized publication (or explicit rejection) is recorded, the published artifact matches the approved candidate, consumer smoke passes, and all MVP constraints are evidenced. Phase 18 may begin only from this approved baseline.

## Status

NOT STARTED.
