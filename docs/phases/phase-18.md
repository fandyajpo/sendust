# Phase 18 — Post-MVP

**Phase status:** NOT STARTED  
**Phase dependency:** Phase 17  
**Companion references:** [release strategy](../engineering/release-strategy.md), [system design](../architecture/system-design.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md), [reliability strategy](../engineering/reliability-strategy.md)

## Objective

Establish a controlled post-MVP backlog and first hardening cycle without weakening or silently changing the approved MVP semantics. Post-MVP work must be evidence-driven, separately gated, and must not retroactively turn MVP guarantees into delivery or retry promises.

## Scope

In scope: post-MVP intake, feedback triage, candidate feature evaluation (SMTPUTF8, AbortSignal, connection pooling research, platform expansion), compatibility policy, and the next-cycle gate. Out of scope: implementing post-MVP features in this phase, changing the published MVP, and reopening frozen constraints without an explicit ADR.

## Tasks

### P18-001 — Collect post-MVP feedback and defect intake

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P17-005
- **Description:** Define channels and templates for consumer feedback, bug reports, security reports, and feature requests. Triage against the proposed MVP contract: relay-accepted semantics, explicit SMTP refusal versus unknown-after-DATA, BUSY, no internal queue/worker/adapter, no automatic retry, caps, attachment policy, TLS verification, and support matrix. Separate defects from feature requests and unsupported-use reports.
- **Evidence criteria:** Intake template captures version, platform, runtime version, relay behavior, result status, and redacted diagnostics. Every item is classified as defect, documentation gap, feature request, or unsupported use. Security reports have a private path. No item is accepted by changing MVP semantics silently.

### P18-002 — Evaluate SMTPUTF8, AbortSignal, and platform expansion candidates

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P18-001
- **Description:** Research post-MVP candidates without committing implementation: SMTPUTF8 support, AbortSignal integration, additional Node.js, Bun, and Deno versions/platforms, and richer recipient reporting if lettre exposes it. Assess compatibility, security, API impact, and test burden. Preserve ASCII local parts and IDNA domains for MVP compatibility.
- **Evidence criteria:** Candidate matrix lists benefit, risk, compatibility, required ADR, and evidence needed. SMTPUTF8 remains explicitly post-MVP until an ADR proves relay support and encoding behavior. AbortSignal must map to bounded close semantics, not required shutdown. Platform expansion requires lifecycle and distribution evidence.

### P18-003 — Evaluate connection pooling as separate future research; queue/job orchestration is held outside the roadmap

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P18-001
- **Description:** Investigate whether connection pooling could be offered in a future release without violating MVP lifecycle and BUSY semantics. Analyze session health, concurrency, relay behavior, resource limits, and close/drain interaction. Retry policy may be researched only as a separate caller-owned policy; no automatic retry implementation is included. Queueing, workers, persistence, scheduling, and queue adapters are not part of this phase or the current roadmap.
- **Evidence criteria:** Research record states MVP remains one-connection-per-send with no pooling and no automatic retry. Any pooling proposal includes lifecycle, concurrency, resource, and compatibility analysis. No queue architecture or adapter contract is produced. No code change occurs in this phase.

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

The planning baseline for this phase, subject to completion of its approval gates, is: two crates with core free of Node, `TS -> napi -> core -> lettre`, `lettre 0.11.23`, rustls `ring`/`webpki-roots` with mandatory verification, no custom SMTP unless previously justified, one connection per send, no automatic retries, relay-accepted success, explicit SMTP refusal as `rejected`, genuinely indeterminate final disposition as `unknown`, one active send with fail-fast `BUSY`, no internal queue/worker/scheduler/persistence/adapter, explicit close bounded by 125s without GC dependence, finite timeouts (connect 10s, operation 30s, total 120s, close 125s), copied buffers, opt-in regular-file attachments with no URLs, caps of 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw, and 30 MiB wire, ASCII local parts with IDNA domains and Unicode display/body, SMTPUTF8 post-MVP, and optional AbortSignal. Post-MVP proposals must preserve these defaults or require an explicit, separately approved breaking-change ADR.

## Dependencies

Phase 18 requires Phase 17 approval. P18-001 depends on P17-005; P18-002 and P18-003 depend on P18-001; P18-004 depends on P18-002 and P18-003; P18-005 depends on all. No task in this phase changes the published MVP.

## Risks and edge cases

Post-MVP enthusiasm can erode MVP honesty, especially around retry, pooling, delivery confirmation, and SMTPUTF8. Feedback may conflate relay acceptance with delivery. A “small” AbortSignal change can introduce unsafe cancellation. Pooling can break one-active-send and close semantics. Retry can duplicate mail after unknown; automatic retry is not in the current roadmap. Platform expansion can outpace lifecycle testing. Deprecation policy can be ignored under release pressure.

## Testing requirements

Post-MVP work is research and intake only; no product tests are added in this phase. Validate intake templates with redacted diagnostics, candidate matrices with evidence links, and policy checks that reject silent semantic changes. Verify the published MVP artifact and hashes are unchanged. If prototypes are used for research, they must run in isolated local fixtures and must not be published or merged into the MVP line.

## Acceptance criteria

- Feedback and defects are triaged against the approved contract.
- Candidate features have evidence, risks, and required ADRs; none are implemented.
- Pooling remains disabled by default and requires explicit future approval.
- No queue/worker/adapter design or implementation is introduced.
- Compatibility/deprecation policy protects MVP semantics.
- Next-cycle backlog is prioritized and the published MVP remains unchanged.

## Deliverables

Feedback intake templates, triage log, post-MVP candidate matrix, connection pooling research record, compatibility/deprecation policy, and prioritized next-cycle backlog with handoff criteria.

## Completion gate

`PHASE-18` is approved only when P18-005 proves a complete, prioritized post-MVP backlog with no MVP semantic or artifact change and explicit entry criteria for future work. This gate closes the planning program; it does not authorize post-MVP implementation.

## Status

NOT STARTED.
