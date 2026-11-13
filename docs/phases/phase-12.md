# Phase 12 — QA

**Phase status:** NOT STARTED  
**Phase dependency:** Phase 11  
**Companion references:** [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md), [reliability strategy](../engineering/reliability-strategy.md), [performance strategy](../engineering/performance-strategy.md), [system design](../architecture/system-design.md)

## Objective

Prove the integrated MVP through traceable test coverage, deterministic fault fixtures, cross-runtime checks, and contract conformance. QA must validate behavior, not merely run suites.

## Scope

In scope: test plan and traceability, core unit/integration coverage, binding and TypeScript contract tests, fault/fixture completeness, candidate Node matrix, regression triage, and QA exit evidence. Out of scope: performance benchmarking (Phase 13), packaging (Phase 14), release publication (Phase 16), and adding new product behavior.

## Tasks

### P12-001 — Build the requirements-to-test traceability matrix

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P11-005
- **Description:** Map every constraint, status, error, limit, lifecycle rule, and security abuse case to an automated test and evidence artifact. Identify gaps where only manual review exists. Include the recipient ADR, unknown-after-DATA, BUSY, close deadline, buffer copy, attachment opt-in, TLS failures, and caps.
- **Evidence criteria:** Matrix has no unowned mandatory requirement. Each row names test file, fixture, and pass condition. Gaps are closed or explicitly accepted with rationale before the gate.

### P12-002 — Execute core and binding integration coverage

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P12-001
- **Description:** Run Rust core unit/integration tests and napi binding tests across the full fake SMTP/TLS matrix. Assert exact command traces, results, deadlines, and cleanup. Include mixed RCPT, explicit final DATA `451` rejection, missing-final-reply-after-DATA unknown, TLS certificate failures, auth failure, resets, stalls, and limit boundaries.
- **Evidence criteria:** Coverage report identifies tested modules and untested critical branches. All fault fixtures pass. Core tests run without Node. Binding tests show no semantic drift. Any skipped test has a named reason and owner.

### P12-003 — Execute TypeScript contract, lifecycle, and error tests

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P12-001, P09-005
- **Description:** Validate the public API with strict type tests and runtime tests: accepted/rejected/unknown recipient outcomes/BUSY, error classes and codes, redaction, close idempotence, send-after-close, concurrent send, attachment opt-in, and no-retry transaction counts. Include candidate Node 22/24/26 where available.
- **Evidence criteria:** Type tests fail on contract-breaking changes. Runtime matrix passes for all statuses and errors. BUSY and unknown tests assert transaction counts. Redaction scans pass. Unsupported Node candidates are documented, not silently skipped.

### P12-004 — Run fixture completeness and regression triage

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P12-002, P12-003
- **Description:** Audit fixture coverage against Phase 00/10/11 catalogs. Add missing boundary or fault cases, remove nondeterministic sleeps, and triage every failure as product bug, fixture bug, or accepted limitation. Re-run the full suite to confirm stability.
- **Evidence criteria:** Fixture checklist is complete for success, rejection, mixed RCPT, unknown, TLS, auth, resets, stalls, close, limits, attachments, injection, buffer mutation, GC, and BUSY. Failures have root causes and dispositions. Two consecutive full runs pass.

### P12-005 — QA exit gate and evidence package

- **Size:** S
- **Status:** NOT STARTED
- **Prerequisites:** P12-001, P12-002, P12-003, P12-004
- **Description:** Assemble QA evidence: traceability matrix, test reports, coverage, candidate-runtime results, fixture audit, and open-risk list. Confirm no unresolved high-severity defect and no contract deviation.
- **Evidence criteria:** Gate package is reproducible from documented commands. Every mandatory requirement is tested. No high-severity bug is open. Phase 13 and Phase 16 may consume P12-005; Phase 14 final matrix uses it via task dependencies.

## Technical implementation details

QA validates the frozen architecture without changing it: two crates, core without Node, `TS -> napi -> core -> lettre`, `lettre 0.11.23`, rustls `ring`/`webpki-roots` with verification always on. Success is relay acceptance; explicit negative final DATA replies are `rejected`; genuinely missing/indeterminate final replies after DATA are `unknown`, with no retry. One active send returns `BUSY`; no queue or pooling. Close is explicit, bounded by 125s, and not GC-dependent. Timeouts are connect 10s, operation 30s, total 120s. Caps are 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw, 30 MiB wire. Filesystem attachments are opt-in and regular-file only; no URLs. Buffers are copied before async. Tests use loopback fake SMTP/TLS only. Candidate Node 22/24/26 are validated, not assumed.

## Dependencies

Phase 12 requires Phase 11. P12-001 depends on P11-005; P12-002 depends on P12-001; P12-003 adds P09-005; P12-004 depends on P12-002/003; P12-005 depends on all. Phase 13 and Phase 16 require P12-005. Phase 14 final matrix depends on P12-005 and P13-005 through task dependencies, not phase cycles.

## Risks and edge cases

Coverage can be high while critical branches remain untested; traceability must target semantics. Flaky timing tests can hide races. Candidate Node versions may be unavailable; absence must be explicit. Fixture gaps around mixed RCPT and post-DATA ambiguity are especially dangerous. A green suite can still miss resource leaks without repeated runs.

## Testing requirements

Run Rust core tests, napi binding tests, and TypeScript contract/runtime tests. Required fixtures: all-recipient success; all RCPT `550`; mixed `250`/`550`; explicit final DATA `451`; missing final reply after DATA terminator; auth `535`; self-signed, expired, wrong-host, incomplete-chain, malformed TLS; connection refused; greeting/EHLO/AUTH/MAIL/RCPT/DATA/final resets; stalls at 10s/30s/120s/125s; abrupt close; malformed reply; 100/101 recipients; 10 MiB, 20 MiB, 30 MiB boundaries; attachment special files; filesystem-disabled default; CRLF/NUL injection; buffer mutation/detachment; forced GC; concurrent BUSY; double close; send-after-close. Assert exact command traces and cleanup. Execute twice for stability.

## Acceptance criteria

- Every mandatory requirement maps to a passing automated test or accepted manual evidence.
- Core, binding, and TypeScript suites pass with exact semantic assertions.
- Candidate Node results are recorded honestly.
- Fixture catalog is complete and deterministic; repeated runs are stable.
- No high-severity defect or contract deviation remains.

## Deliverables

Traceability matrix, core/binding/TypeScript test reports, coverage report, candidate-runtime matrix, fixture audit, defect dispositions, and QA exit package.

## Completion gate

`PHASE-12` is approved only when P12-005 proves traceable coverage, deterministic passing suites, honest candidate-runtime results, and no unresolved high-severity defect. Phase 13 and Phase 16 may begin after approval.

## Modular architecture audit

P12 must verify the ADR-0016 module ownership table, allowed dependency graph, domain isolation from lettre/SMTP/napi/runtime APIs, SMTP adapter containment, absence of public plugin registries or duplicate policy, and focused unit/integration test ownership. Any boundary violation is a release-blocking architecture defect, not a style preference.

## Status

NOT STARTED.
