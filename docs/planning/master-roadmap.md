# Master Roadmap

**Planning date:** 2026-11-13.  
**Repository state at planning time:** planning and architecture documentation; application implementation was not present in the audited snapshot.  
**Execution rule:** no phase starts before its predecessor’s completion gate is approved. P00–P02 are completed with reviewed gate evidence; P03–P18 remain `NOT STARTED`.

This roadmap summarizes the 19 phase plans. Detailed tasks, tests, acceptance criteria, and deliverables live in [docs/phases](../phases/). Requirements live in [requirements.md](../product/requirements.md).

## 1. Lifecycle at a glance

| Phase | Outcome | Gate |
| --- | --- | --- |
| P00 Discovery | Problem, constraints, semantics, threat boundary, and open questions fixed | PHASE-00 |
| P01 Research | lettre, rustls, recipient observability, and Node lifecycle resolved with evidence | PHASE-01 |
| P02 Architecture | Approve two-crate design, status/error contract, lifecycle, ownership, and limits | PHASE-02 |
| P03 Repository | Workspace, toolchain, policies, and deterministic test seams exist | PHASE-03 |
| P04 Rust core | Validation, message model, state machine, and lettre orchestration compile and pass core tests | PHASE-04 |
| P05 Binding | Node-API 8 adapter exposes core with owned memory and stable mapping | PHASE-05 |
| P06 SMTP | Relay sessions, TLS/auth, recipient policy, and response normalization pass the fault matrix | PHASE-06 |
| P07 MIME | Headers, alternative bodies, filenames, and deterministic serialization conform to MIME | PHASE-07 |
| P08 Attachments | Opt-in files, safe reads, caps, and MIME integration pass the attachment matrix | PHASE-08 |
| P09 Public API | Approve TypeScript transport, send, close, results, and errors against the architecture contract | PHASE-09 |
| P10 Reliability | Races, timeout interaction, cleanup, and fault injection are hardened | PHASE-10 |
| P11 Security | Threat model, TLS/auth, injection, attachments, ownership, and redaction are signed off | PHASE-11 |
| P12 QA | Requirements-to-test traceability and full integration coverage pass | PHASE-12 |
| P13 Performance | Budgets defined and measured; no unexplained regression | PHASE-13 |
| P14 Distribution | Clean install and load on every declared target | PHASE-14 |
| P15 Documentation | User, API, limits, errors, security, and maintainer docs are accurate | PHASE-15 |
| P16 Release engineering | Reproducible RC with provenance, notes, checklist, and rollback plan | PHASE-16 |
| P17 RC and MVP approval | Final acceptance, authorized publication, and post-publication verification | PHASE-17 |
| P18 Post-MVP | Feedback intake and researched next-cycle candidates | PHASE-18 |

## 2. Phase summaries

### P00 — Discovery

<badge>COMPLETED — QG-00 approved</badge>

Established the non-negotiable semantics: relay acceptance is not delivery, one send per transport, explicit bounded close, no pooling/retry, ASCII local parts with IDNA domains, opt-in filesystem attachments, and hard caps. Produces the constraint ledger, provenance table, threat boundary, and gate handoff. No code.

**Key risk:** ambiguous semantics discovered late. **Mitigation:** every constraint gets an owner and validation task before research starts.

### P01 — Research

<badge>COMPLETED — QG-01 approved with downstream implementation gates</badge>

Resolved the source/API questions that can invalidate the architecture:

1. Which lettre 0.11.23 async API to use.
2. Whether rustls `ring` + `webpki-roots` composes and keeps verification mandatory.
3. Whether per-recipient RCPT status and exact timeout stages are observable.
4. Whether the napi-managed Tokio runtime and Node-API 8 behave on Node 22/24/26.

**Key risk:** relying on high-level transport semantics. **Mitigation:** selected lettre's public lower-level API and explicit RCPT response checks; P06-003 still validates with a fake relay.

### P02 — Architecture

<badge>COMPLETED — QG-02 approved at design level</badge>

Approved the two-crate boundary, dependency direction, public result/error contract, state machine, timeout model, ownership rules, attachment policy, and caps. Produced ADR decisions and a constraint-to-task traceability matrix. No application implementation.

**Key risk:** a “thin” binding absorbing policy. **Mitigation:** core-only tests and a negative dependency check.

### P03 — Repository

Creates the Cargo workspace with exactly two crates, the pnpm/TypeScript workspace, pinned toolchain, formatting/linting/type-check/test commands, dependency/license policy, and deterministic fake SMTP/TLS harness seams. Baseline must pass with placeholder-free skeletons.

**Key risk:** test seams added later causing rewrites. **Mitigation:** harness seams are a Phase 03 deliverable.

### P04 — Rust core

Implements the error taxonomy and deterministic validation order, address/header/IDNA policy, message model and size accounting, transport state machine with deadlines and BUSY, and lettre-backed orchestration. Core tests run without Node.

**Key risk:** state machine races and nondeterministic error precedence. **Mitigation:** explicit transition table, injected clock, ordered validation tests.

### P05 — Binding

Defines the Node-API 8 surface, copies all JS inputs, runs async tasks on the napi-managed Tokio runtime, maps results/errors, and passes lifecycle tests including forced GC and close deadlines. No protocol logic in the adapter.

**Key risk:** borrowed memory and finalizer races. **Mitigation:** copy-before-async rule and forced-GC tests.

### P06 — SMTP

Configures lettre relay sessions, enforces mandatory TLS verification and authentication semantics, implements exactly one recipient policy from P01-003, normalizes responses, and passes the full SMTP fault matrix with exact command traces.

**Key risk:** retrying after DATA or splitting per recipient. **Mitigation:** transaction-count assertions and a forbidden-path audit.

### P07 — MIME

Implements the header model with RFC 2047 encoding and CRLF safety, text/HTML alternative bodies, filename/content-type policy, deterministic serialization, and wire-size parity. Conformance is verified with an independent parser.

**Key risk:** encoding and folding edge cases. **Mitigation:** golden fixtures and `mail-parser` conformance tests.

### P08 — Attachments

Defines the opt-in filesystem contract, implements safe regular-file validation and bounded reads, enforces per-file/aggregate/wire caps, and integrates copied bytes into MIME. Passes the attachment fault matrix.

**Key risk:** TOCTOU, symlinks, FIFOs, and file growth during read. **Mitigation:** open-then-fstat, no-follow, cap-probe read loop.

### P09 — Public API

Implements the TypeScript transport, `send`, `close`, state, result mapping, error classes, redaction, and module entry points. Adds type tests and runnable fake-relay examples. No semantic drift from core.

**Key risk:** wrapper reinterpreting statuses or adding hidden convenience. **Mitigation:** contract gate with compile-time and runtime tests.

### P10 — Reliability

Hardens lifecycle races, timeout-budget interaction, bounded close drain, resource cleanup, and leak prevention. Builds the fault-injection suite that later phases reuse.

**Key risk:** close/send races and leaked tasks. **Mitigation:** deterministic race matrix and task/connection accounting.

### P11 — Security

Executes the threat model and audits TLS/auth, validation/MIME/attachment injection, memory ownership, redaction, and denial-of-service bounds. Remediation is required before QA.

**Key risk:** redaction gaps and verification bypass. **Mitigation:** serialized-error scans and source audit for bypass paths.

### P12 — QA

Builds the requirements-to-test traceability matrix, runs core/binding/TypeScript integration coverage, verifies fixture completeness, and triages regressions. Produces the QA evidence package.

**Key risk:** untested requirements. **Mitigation:** matrix gate; a requirement without a test blocks exit.

### P13 — Performance

Defines budgets and measurement method, benchmarks the send path, MIME serialization, attachment reads/copies, and lifecycle overhead. Produces a regression report. No premature optimization.

**Key risk:** benchmarking noise and optimizing the wrong path. **Mitigation:** fixed environment, repeated runs, budget thresholds approved before measurement.

### P14 — Distribution

Runs an early clean-consumer smoke, defines package metadata/exports/artifact layout, specifies the platform matrix, validates clean installs and runtime loading, and passes the final distribution gate using QA and performance evidence.

**Key risk:** silent wrong-architecture resolution and non-atomic publication. **Mitigation:** loader diagnostics, artifact manifest, per-target install tests.

### P15 — Documentation

Writes quick start, semantic contract, lifecycle/timeouts/limits, address/MIME/attachment behavior, error catalog, security/operations guidance, and a maintainer guide. Accuracy gate checks every example against shipped behavior.

**Key risk:** documentation implying delivery or unsupported platforms. **Mitigation:** link checks, executable examples, support-matrix review.

### P16 — Release engineering

Defines versioning/tagging/changelog, assembles reproducible builds with provenance and SBOM, drafts release notes and limitations, creates the release checklist and rollback plan, and hands off an RC without publishing.

**Key risk:** nondeterministic builds and stale notes. **Mitigation:** double-build hash comparison and evidence-linked checklist.

### P17 — RC and MVP approval

Verifies RC identity and evidence, runs final MVP acceptance against the constraint ledger, authorizes publication, publishes, verifies the registry and platform packages, and records rollback readiness.

**Key risk:** partial publication. **Mitigation:** artifact-completeness gate and documented recovery procedure.

### P18 — Post-MVP

Collects feedback, researches SMTPUTF8, AbortSignal, platform expansion, connection pooling, streaming, and additional transports, and defines the next compatibility/deprecation policy. Queue/job orchestration and queue adapters are held outside this roadmap. No post-MVP feature enters the codebase without a new requirement, task, and test.

## 3. Effort profile

Relative effort only; no dates.

| Phase | Effort | Dominant cost |
| --- | --- | --- |
| P00 | S | Writing and review |
| P01 | M | Source inspection and probes |
| P02 | M | Contract design and ADR review |
| P03 | M | Workspace and harness setup |
| P04 | L | Core types, state machine, orchestration |
| P05 | L | Node-API ownership and async lifecycle |
| P06 | XL | SMTP fault matrix and recipient policy |
| P07 | L | MIME conformance |
| P08 | L | Filesystem safety and caps |
| P09 | L | Public contract and errors |
| P10 | L | Race and failure hardening |
| P11 | M | Audits and remediation |
| P12 | L | Full traceability and coverage |
| P13 | M | Benchmarks and report |
| P14 | L | Cross-platform builds and installs |
| P15 | M | Documentation set |
| P16 | L | Reproducibility and release assets |
| P17 | M | Verification and publication |
| P18 | S | Intake and research planning |

The critical path is P00 (completed) → P01 (completed) → P02 (completed) → P03 → P04 → P06 → P09 → P10 → P11 → P12 → P13 → P16 → P17. See [dependency-graph.md](dependency-graph.md).

## 4. What “done” means

A phase is done only when its completion gate is approved with linked evidence. A task is done only per [definition-of-done.md](definition-of-done.md). The project is not “mostly done” at any point; status is binary per task and per gate.

## 5. Explicit non-goals for the roadmap

No deadlines, no story points, no velocity, no team-size assumptions, no enterprise architecture, no features outside [mvp-scope.md](../product/mvp-scope.md). Post-MVP work is planned only as research, not as committed scope. Queueing, workers, persistence, scheduling, retry orchestration, and queue adapters are not planned deliverables.
