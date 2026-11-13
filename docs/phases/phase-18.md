# Phase 18 — Post-MVP

**Phase status:** NOT STARTED  
**Phase dependency:** Phase 17  
**Companion references:** [release strategy](../engineering/release-strategy.md), [system design](../architecture/system-design.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md), [reliability strategy](../engineering/reliability-strategy.md)

## Objective

Establish a controlled post-MVP backlog and first hardening cycle without weakening or silently changing the approved MVP semantics. Post-MVP work must be evidence-driven, separately gated, and must not retroactively turn MVP guarantees into delivery or retry promises.

## Scope

In scope: post-MVP intake, feedback triage, candidate feature evaluation (SMTPUTF8, AbortSignal, pooling/retry research, platform expansion), compatibility policy, and the next-cycle gate. Out of scope: implementing post-MVP features in this phase, changing the published MVP, and reopening frozen constraints without an explicit ADR.

## Tasks

### P18-001 — Collect post-MVP feedback and defect intake

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P17-005
- **Description:** Define channels and templates for consumer feedback, bug reports, security reports, and feature requests. Triage against the approved MVP contract: relay-accepted semantics, unknown-after-DATA, BUSY, no queue/retry, caps, attachment policy, TLS verification, and support matrix. Separate defects from feature requests and unsupported-use reports.
- **Evidence criteria:** Intake template captures version, platform, Node version, relay behavior, result status, and redacted diagnostics. Every item is classified as defect, documentation gap, feature request, or unsupported use. Security reports have a private path. No item is accepted by changing MVP semantics silently.

### P18-002 — Evaluate SMTPUTF8, AbortSignal, and platform expansion candidates

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P18-001
- **Description:** Research post-MVP candidates without committing implementation: SMTPUTF8 support, AbortSignal integration, additional Node versions/platforms, and richer recipient reporting if lettre exposes it. Assess compatibility, security, API impact, and test burden. Preserve ASCII local parts and IDNA domains for MVP compatibility.
- **Evidence criteria:** Candidate matrix lists benefit, risk, compatibility, required ADR, and evidence needed. SMTPUTF8 remains explicitly post-MVP until an ADR proves relay support and encoding behavior. AbortSignal must map to bounded close semantics, not required shutdown. Platform expansion requires lifecycle and distribution evidence.

### P18-003 — Evaluate pooling and retry only as explicit opt-in research

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P18-001
- **Description:** Investigate whether connection pooling or retry could be offered later as explicit opt-in behavior without violating MVP defaults. Analyze duplicate-send risk, unknown-after-DATA, relay idempotency, queue semantics, BUSY, and close/drain interaction. Do not implement or enable either in the MVP line.
- **Evidence criteria:** Research record states MVP remains no-pooling/no-retry. Any proposal includes duplicate-send analysis, idempotency requirements, opt-in API shape, and failure semantics. Automatic retry after DATA remains rejected unless a future ADR proves safe and explicit. No code change occurs in this phase.

### P18-004 — Define the next compatibility and deprecation policy

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P18-002, P18-003
- **Description:** Define how post-MVP releases preserve status semantics, error codes, close behavior, caps, TLS posture, and support matrix. Specify deprecation notice periods, migration guidance, and the rule that relaxing limits or adding features requires documentation and tests. Clarify that delivery confirmation is not planned without a separate product decision.
- **Evidence criteria:** Policy covers semantic versioning, deprecation, support windows, and breaking-change approval. It forbids silently enabling pooling, retry, SMTPUTF8, or URL attachments. Each future feature requires an ADR, tests, docs, and release-gate evidence. MVP constraints remain the baseline.

### P18-005 — Post-MVP intake gate and next-cycle handoff

- **Size:** S
- **Status:** NOT STARTED
- **Prerequisites:** P18-001, P18-002, P18-003, P18-004
- **Description:** Consolidate triage, research, and policy into a prioritized next-cycle backlog. Separate must-fix defects from optional features and rejected ideas. Confirm no post-MVP item has altered the approved MVP artifact or semantics. Hand off with explicit entry criteria for any future phase.
- **Evidence criteria:** Backlog has priority, rationale, dependencies, required ADR, and evidence criteria for each item. Rejected items include reasons. MVP baseline and published hashes remain unchanged. The gate records that no post-MVP implementation started in this phase.

## Technical implementation details

The approved baseline is immutable for this phase: two crates with core free of Node, `TS -> napi -> core -> lettre`, `lettre 0.11.23`, rustls `ring`/`webpki-roots` with mandatory verification, no custom SMTP unless previously justified, no pooling, no automatic retries, relay-accepted success, unknown-after-DATA without retry, one active send with fail-fast `BUSY`, no queue, explicit close bounded by 125s without GC dependence, finite timeouts (connect 10s, operation 30s, total 120s, close 125s), copied buffers, opt-in regular-file attachments with no URLs, caps of 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw, and 30 MiB wire, ASCII local parts with IDNA domains and Unicode display/body, SMTPUTF8 post-MVP, and optional AbortSignal. Post-MVP proposals must preserve these defaults or require an explicit, separately approved breaking-change ADR.

## Dependencies

Phase 18 requires Phase 17 approval. P18-001 depends on P17-005; P18-002 and P18-003 depend on P18-001; P18-004 depends on P18-002 and P18-003; P18-005 depends on all. No task in this phase changes the published MVP.

## Risks and edge cases

Post-MVP enthusiasm can erode MVP honesty, especially around retry, pooling, delivery confirmation, and SMTPUTF8. Feedback may conflate relay acceptance with delivery. A “small” AbortSignal change can introduce unsafe cancellation. Pooling can break one-active-send and close semantics. Retry can duplicate mail after unknown. Platform expansion can outpace lifecycle testing. Deprecation policy can be ignored under release pressure.

## Testing requirements

Post-MVP work is research and intake only; no product tests are added in this phase. Validate intake templates with redacted diagnostics, candidate matrices with evidence links, and policy checks that reject silent semantic changes. Verify the published MVP artifact and hashes are unchanged. If prototypes are used for research, they must run in isolated local fixtures and must not be published or merged into the MVP line.

## Acceptance criteria

- Feedback and defects are triaged against the approved contract.
- Candidate features have evidence, risks, and required ADRs; none are implemented.
- Pooling and retry remain disabled by default and require explicit future approval.
- Compatibility/deprecation policy protects MVP semantics.
- Next-cycle backlog is prioritized and the published MVP remains unchanged.

## Deliverables

Feedback intake templates, triage log, post-MVP candidate matrix, pooling/retry research record, compatibility/deprecation policy, and prioritized next-cycle backlog with handoff criteria.

## Completion gate

`PHASE-18` is approved only when P18-005 proves a complete, prioritized post-MVP backlog with no MVP semantic or artifact change and explicit entry criteria for future work. This gate closes the planning program; it does not authorize post-MVP implementation.

## Status

NOT STARTED.
