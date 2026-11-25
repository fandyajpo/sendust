# Documentation Index

**Sendust — Standalone Native Email Delivery Engine for Node.js, Bun, and Deno**  
Planning baseline: 2026-11-13.

This documentation set is the source of truth for product scope, requirements, approved architecture design, decisions, and execution planning. Repository-wide AI/contributor behavior is governed by [AGENTS.md](../AGENTS.md) and the [AI implementation contract](engineering/ai-implementation-contract.md). QG-00, QG-01, and QG-02 are approved. The two-crate workspace, the native SMTP engine (P06), MIME composition (P07), the Node-API binding, the runtime-neutral TypeScript facade, attachment file policy (P08), public API validation (P09), runtime and platform support claims, and publication are complete. `@sendust/ts` is published through npm trusted publishing with provenance, on seven native targets: `1.2.0` adds DKIM signing on top of the musl targets and XOAUTH2 in `1.1.0` ([release notes](releases/1.2.0.md), [gate record](gates/release-1.2.0.md)), and `1.2.1` publishes the npm-page metadata ([release notes](releases/1.2.1.md)). `1.3.0` fixes the source audit's findings — the `failed` state, `SIZE` enforcement, the `AUTH` line bound, and six wrapper reliability gaps ([release notes](releases/1.3.0.md), [gate record](gates/release-1.3.0.md)) — and it is published, with `latest` resolving it. `1.4.0` resolves the audit's last four findings — the two documented error details are populated, the dead `lifecycle` and `validation` modules are deleted, and `PROTOCOL_REJECTED` is deprecated ([release notes](releases/1.4.0.md), [gate record](gates/release-1.4.0.md)) — and is prepared.

## 1. Start here

| Order | Document | Why |
| --- | --- | --- |
| 1 | [Product vision](product/vision.md) | Problem, users, principles, positioning |
| 2 | [Requirements](product/requirements.md) | Constraints and functional/non-functional requirements |
| 3 | [MVP scope](product/mvp-scope.md) | Included, excluded, and success conditions |
| 4 | [Discovery brief](discovery/brief.md) | Falsifiable problem statement and product boundary |
| 5 | [Constraint ledger](discovery/constraint-ledger.md) | C-01..C-40 traceability |
| 6 | [Threat model](discovery/threat-model.md) | Assets, trust boundaries, and abuse cases |
| 7 | [lettre source validation](research/lettre-0.11.23-validation.md) | Pinned API and feature research |
| 8 | [Node runtime validation plan](research/node-runtime-validation-plan.md) | Node-API 8 candidate matrix and lifecycle probes |
| 8a | [Multi-runtime compatibility research](research/multi-runtime-compatibility.md) | Node.js, Bun, Deno and native platform evidence |
| 8b | [Toolchain baseline](engineering/toolchain-baseline.md) | Exact runtime/tool versions and canonical validation commands |
| 9 | [Architecture overview](architecture/overview.md) | Approved architecture shape and boundaries |
| 10 | [System design](architecture/system-design.md) | Modules, types, state machine, test seams |
| 11 | [Public API contract](architecture/public-api.md) | Approved design contract; implementation gate P09 |
| 12 | [Data flow](architecture/data-flow.md) | Send and close across layers |
| 13 | [Error model](architecture/error-model.md) | Outcomes, error codes, redaction |
| 14 | [Architecture approval matrix](architecture/approval-matrix.md) | Approved decisions and downstream validations |
| 15 | [ADR index](architecture/decisions/README.md) | Architecture decision records |
| 16 | [Master roadmap](planning/master-roadmap.md) | All 19 phases and outcomes |
| 17 | [Phase index](planning/phase-index.md) | Entry point to every phase plan |
| 18 | [Task registry](planning/task-registry.md) | 95 phase tasks plus 7 architecture-amendment tasks and prerequisites |
| 19 | [Dependency graph](planning/dependency-graph.md) | Phase DAG and critical path |
| 20 | [Milestones](planning/milestones.md) | M0–M7 exit criteria |
| 21 | [Quality gates](planning/quality-gates.md) | QG-00–QG-17 plus QG-03A, QG-A01, QG-A02 |
| 22 | [Definition of done](planning/definition-of-done.md) | Evidence rules |
| 23 | [Risk registry](planning/risk-registry.md) | Risk IDs, mitigations, owners |
| 24 | [Dependency research](engineering/dependency-research.md) | Observed facts and sources |
| 25 | [Testing strategy](engineering/testing-strategy.md) | Test layers, fixtures, fault matrix |
| 26 | [Security strategy](engineering/security-strategy.md) | TLS, injection, redaction, supply chain |
| 27 | [Reliability strategy](engineering/reliability-strategy.md) | Lifecycle, timeouts, races, cleanup |
| 28 | [Performance strategy](engineering/performance-strategy.md) | Budgets and measurement |
| 29 | [Distribution strategy](engineering/distribution-strategy.md) | Platform matrix and native artifacts |
| 30 | [Release strategy](engineering/release-strategy.md) | Versioning, RC, publication, rollback |
| 31 | [QG-00 discovery gate](gates/phase-00-discovery.md) | Discovery approval evidence |
| 32 | [QG-01 research gate](gates/phase-01-research.md) | Research approval evidence |
| 33 | [QG-02 architecture gate](gates/phase-02-architecture-approval.md) | Original architecture approval evidence |
| 34 | [Architecture Amendment A01](gates/architecture-amendment-a01.md) | Multi-runtime and cross-platform design amendment |
| 35 | [Architecture Amendment A02](gates/architecture-amendment-a02.md) | Modular monolith and internal module boundaries |
| 36 | [AI implementation contract](engineering/ai-implementation-contract.md) | Normative workflow, documentation synchronization, static-review and evidence rules |
| 37 | [Toolchain baseline](engineering/toolchain-baseline.md) | Exact Rust, Node.js, Bun, Deno, npm, TypeScript, Biome, Vitest versions and canonical commands |
| 38 | [Documentation audit](audits/2026-11-13-documentation-audit.md) | Static consistency, traceability, task graph, links, toolchain, and remaining validation owners |
| 38a | [Engineering audit](audits/2026-11-15-engineering-audit.md) | Read-only code review findings, their remediation, and the owner actions that remain |
| 39 | [Dependency and license policy](engineering/dependency-policy.md) | Pinned dependencies, feature admission, feature prohibitions, lockfile rules, and license review |
| 40 | [Test harness contract](engineering/test-harness-contract.md) | Fake SMTP/TLS seams, safety rules, and the fixture catalog |
| 41 | [QG-03 repository baseline record](gates/phase-03-repository-baseline.md) | P03 gate checklist, evidence, and approved decision |
| 42 | [PHASE-04 core record](gates/phase-04-core.md) | P04 gate checklist, evidence, and approved decision |
| 43 | [PHASE-05 binding record](gates/phase-05-binding.md) | P05 gate checklist, evidence, and approved decision |
| 44 | [PHASE-06 SMTP engine record](gates/phase-06.md) | P06 gate checklist, native engine evidence, and approved decision |
| 45 | [PHASE-07 MIME record](gates/phase-07.md) | P07 gate checklist, conformance evidence, and approved decision |
| 46 | [PHASE-08 attachments record](gates/phase-08.md) | P08 gate checklist, attachment fault matrix, and approved decision |
| 47 | [PHASE-09 public API record](gates/phase-09.md) | P09 gate checklist, contract surface lock, and approved decision |
| 48 | [PHASE-10 reliability record](gates/phase-10.md) | P10 gate checklist, fault matrix, stress evidence, and approved decision |
| 49 | [Threat-model closure](security/threat-model-closure.md) | P11-001 abuse-case matrix, boundaries, residual risks, and ownership |
| 50 | [TLS and authentication audit](security/tls-auth-audit.md) | P11-002 configuration, certificate and auth matrix, and dependency proof |
| 51 | [Validation, MIME, and attachment audit](security/injection-attachment-audit.md) | P11-003 injection fixtures, filename and path rules, MIME rendering, attachment reads, and cap placement |
| 52 | [Ownership, redaction, and bounds audit](security/ownership-redaction-audit.md) | P11-004 buffer ownership, redaction matrix, concurrency/deadline bounds, and the panic guard |
| 53 | [PHASE-11 security record](gates/phase-11.md) | P11 gate checklist, abuse-case closure, licence review, evidence, and the approved decision |
| 54 | [Requirements-to-test traceability matrix](qa/traceability-matrix.md) | P12-001 mapping of every constraint, requirement, limit, lifecycle rule, error contract, and abuse case to a test or gate |
| 55 | [Integration coverage report](qa/integration-coverage.md) | P12-002 suite inventory, module map, critical branches, skips, and uncovered gaps |
| 56 | [TypeScript contract report](qa/typescript-contract-report.md) | P12-003 type, error, result, lifecycle, and runtime-matrix results with their scoped gaps |
| 57 | [Fixture completeness audit](qa/fixture-audit.md) | P12-004 fixture checklist, the forced-GC probe, sleep removal, and regression triage |
| 58 | [PHASE-12 QA record](gates/phase-12.md) | P12 gate checklist, suite summary, traceability, runtime matrix, and the approved decision |
| 59 | [Performance budget](qa/performance-budget.md) | P13-001 budget table, measurement method, harness contract, and regression thresholds |
| 60 | [Full repository audit](audits/2026-11-15-full-repository-audit.md) | Repository-wide audit after PHASE-12: seven documentation findings, fixes, verified claims, and open owners |
| 61 | [Core and MIME performance](qa/performance-core-mime.md) | P13-002 serialization results, budget comparison, linearity, and limitations |
| 62 | [Attachment performance](qa/performance-attachments.md) | P13-003 read, copy, serialization, and peak-memory results with their bounds |
| 63 | [Lifecycle performance](qa/performance-lifecycle.md) | P13-004 BUSY, close, concurrency, and descriptor results |
| 64 | [PHASE-13 performance record](gates/phase-13.md) | P13 gate checklist, consolidated budget results, regression thresholds, and the approved decision |
| 65 | [Package metadata contract](qa/package-metadata-contract.md) | P14-002 fields, exports, native layout, loader order, and the dual-format build |
| 66 | [Platform matrix](qa/platform-matrix.md) | P14-003 target matrix, per-platform package layout, build recipes, and unsupported set |
| 67 | [Platform validation](qa/platform-validation.md) | P14-004 clean-install, load, send, BUSY, close, and negative results per target |
| 68 | [PHASE-14 distribution record](gates/phase-14.md) | QG-14 checklist, support declaration, runtime matrix, findings, and the approved decision |
| 69 | [Quick start and semantic contract](guides/quick-start.md) | P15-001 install steps, first send, accepted/rejected/unknown and BUSY semantics, and the executable sample |
| 70 | [Lifecycle, timeouts, concurrency, and limits](guides/lifecycle-limits.md) | P15-002 state machine, close, BUSY, the four timeout budgets, and the four hard caps with boundary behaviour |
| 71 | [Addresses, MIME, and attachments](guides/addressing-mime-attachments.md) | P15-003 address rules, header safety, body shapes, attachment opt-in and caps, and the content-type map |
| 72 | [Errors, security, and operations](guides/errors-security-operations.md) | P15-004 error catalog with caller actions, redaction, TLS posture, and operational guidance |
| 72a | [DKIM signing](guides/dkim-signing.md) | 1.2.0: what is signed, key creation, the DNS record, verification, rotation, and troubleshooting |
| 73 | [Maintainer guide](guides/maintainer.md) | P15-005 repository layout, commands, test harness, documentation rules, and release inputs |
| 74 | [PHASE-15 documentation record](gates/phase-15.md) | QG-15 checklist, guide coverage, accuracy instruments, findings, and the approved decision |
| 75 | [Versioning, tagging, and changelog policy](engineering/versioning-policy.md) | P16-001 semantic versioning, pre-release and tag format, version sources, changelog, and compatibility promises |
| 76 | [Reproducible build and artifact provenance](engineering/reproducible-build.md) | P16-002 pinned inputs, recipe, determinism evidence, provenance and inventory format, and negative checks |
| 77 | [0.1.0-rc.1 release notes](releases/0.1.0-rc.1.md) | P16-003 candidate notes: support, install, semantics, limits, security, and known limitations |
| 78 | [Release checklist and rollback plan](engineering/release-checklist.md) | P16-004 candidate checks with owners and evidence, plus the publication and rollback boundaries |
| 79 | [PHASE-16 release-engineering record](gates/phase-16.md) | QG-16 checklist, candidate identity and hashes, findings, and the pending handoff decision |
| 80 | [RC verification record](qa/rc-verification.md) | P17-001 candidate identity, artifact contents, evidence completeness, and critical smokes from the candidate artifacts |
| 81 | [MVP acceptance matrix](qa/mvp-acceptance.md) | P17-002 every acceptance item and constraint with evidence, and the owner's approval |
| 82 | [Publication checklist and authorization](engineering/publication-checklist.md) | P17-003 inputs, pre-flight checks, CI publish path, post-publication checks, tag, and rollback |
| 83 | [PHASE-17 record](gates/phase-17.md) | QG-17 checklist, publication evidence, findings, and the approved decision |
| 84 | [Post-MVP intake and defect triage](qa/post-mvp-intake.md) | P18-001 channels, template, classification rules, and the triage log |
| 85 | [Post-MVP candidate matrix](research/post-mvp-candidates.md) | P18-002 SMTPUTF8, `AbortSignal`, pooling pointer, platform expansion, and rejected candidates with their evidence needs |
| 86 | [Connection pooling research](research/connection-pooling.md) | P18-003 the contract reuse must preserve, four options, the evidence an ADR needs, and the decision to defer |
| 87 | [Compatibility and deprecation policy](engineering/compatibility-policy.md) | P18-004 semver levels, the frozen MVP contract, the deprecation process, and the support windows |
| 88 | [Post-MVP backlog and next-cycle handoff](planning/post-mvp-backlog.md) | P18-005 prioritized backlog, rejected items with reasons, and the entry criteria for future work |
| 89 | [PHASE-18 post-MVP closure record](gates/phase-18.md) | QG-18 checklist, phase evidence, findings, and the pending decision |

## 2. Directory map

    docs/
    ├── README.md
    ├── discovery/
    │   ├── brief.md
    │   ├── constraint-ledger.md
    │   └── threat-model.md
    ├── research/
    │   ├── lettre-0.11.23-validation.md
    │   └── node-runtime-validation-plan.md
    ├── gates/
    │   ├── phase-00-discovery.md
    │   ├── phase-01-research.md
    │   └── phase-02-architecture-approval.md
    ├── product/
    ├── architecture/
    │   ├── approval-matrix.md
    │   └── decisions/
    ├── planning/
    ├── phases/
    ├── engineering/
    ├── audits/
    │   ├── 2026-11-13-documentation-audit.md
    │   └── 2026-11-15-engineering-audit.md
    └── decisions/  (pointer to architecture/decisions)

## 3. Status model

P00–P16 are COMPLETED with approved evidence. ADR-0017 supersedes ADR-0002 and selects a Sendust-owned SMTP engine, implemented and approved under PHASE-06; PHASE-07 records MIME composition; PHASE-11 records the security gate; PHASE-12 records QA; PHASE-13 records performance; PHASE-14 records distribution on five native targets; PHASE-15 records the user and maintainer documentation; PHASE-16 records the release-engineering gate and assembles candidate `0.1.0-rc.1` with its hashes and no publication. P17 is COMPLETE: the candidate was verified, the owner approved the MVP and the publication, six packages were published to npm on 2026-11-18 and verified from the registry, and PHASE-17 is approved — the MVP is released. P18 is COMPLETE with the post-MVP intake, triage, candidate research, pooling research, compatibility policy, and backlog; PHASE-18 is approved and the planning program is closed. Every phase task is complete with evidence. A task is COMPLETED only with evidence per the [definition of done](planning/definition-of-done.md). A phase is COMPLETED only when its gate is approved.

| State | Meaning |
| --- | --- |
| NOT STARTED | No work begun |
| IN PROGRESS | Work begun; evidence incomplete |
| BLOCKED | Named blocker with owner |
| IN REVIEW | Deliverable and evidence exist; gate pending |
| COMPLETED | Acceptance criteria met; evidence linked |

## 4. Current state

| Item | State |
| --- | --- |
| Repository | Documentation, architecture planning, and the two-crate workspace with native SMTP, MIME, Node-API binding, and TypeScript facade |
| Application implementation | Core complete (errors, addresses, message model, MIME, orchestration, fault matrix, native SMTP engine); Node-API binding and TypeScript facade implemented and approved under PHASE-05 and PHASE-09; attachment file policy (P08) and public API contract (P09) COMPLETED |
| Toolchain validation | Rust 1.100.0 and the Node.js 22.23.5/24.22.3/26.13.1 jobs pass in Sendust CI |
| QG-00 Discovery | APPROVED |
| QG-01 Research | APPROVED with implementation validation gates |
| QG-02 Architecture | APPROVED at design level |
| P03-001–P03-005 | COMPLETED |
| P04-001–P04-005 | COMPLETED |
| P05-001–P05-005 | COMPLETED |
| P06-001–P06-005 | COMPLETED; PHASE-06 APPROVED (2026-11-14) |
| P07-001–P07-005 | COMPLETED; PHASE-07 APPROVED (2026-11-14) |
| P08-001–P08-005 | COMPLETED; PHASE-08 APPROVED (2026-11-15) |
| P09-001–P09-005 | COMPLETED; PHASE-09 APPROVED (2026-11-15) |
| P10-001–P10-005 | COMPLETED; PHASE-10 APPROVED (2026-11-15) |
| P11-001–P11-005 | COMPLETED; PHASE-11 APPROVED (2026-11-15) |
| P12-001–P12-005 | COMPLETED; PHASE-12 APPROVED (2026-11-15) |
| P13-001–P13-005 | COMPLETED; PHASE-13 APPROVED (2026-11-15) |
| P14-001 | COMPLETED — early clean-consumer smoke |
| P14-002 | COMPLETED — metadata, ESM/CJS entry, publish layout |
| P14-003 | COMPLETED — five-target matrix |
| P14-004 | COMPLETED — per-target install, runtime and behaviour validation |
| P14-005 | COMPLETED — PHASE-14 approved 2026-11-17 |
| P15-001 | COMPLETED — quick start and semantic contract, with an executed sample |
| P15-002 | COMPLETED — lifecycle, timeouts, concurrency, and limits |
| P15-003 | COMPLETED — addresses, MIME, and attachments |
| P15-004 | COMPLETED — error catalog, security, and operations |
| P15-005 | COMPLETED — PHASE-15 approved 2026-11-18 |
| P16-001 | COMPLETED — versioning, changelog, and version consistency check |
| P16-002 | COMPLETED — reproducible build inputs, provenance, and dependency inventory |
| P16-003 | COMPLETED — release notes draft for 0.1.0-rc.1 |
| P16-004 | COMPLETED — release checklist and rollback plan |
| P16-005 | COMPLETED — PHASE-16 approved 2026-11-18 |
| P17-001 | COMPLETED — RC identity, artifacts, and critical smokes verified |
| P17-002 | COMPLETED — MVP acceptance approved 2026-11-18 |
| P17-003 | COMPLETED — publication authorized 2026-11-18 |
| P17-004 | COMPLETED — six packages published to npm 2026-11-18 and verified from the registry |
| P17-005 | COMPLETED — PHASE-17 approved 2026-11-18; MVP released |
| P18-001 | COMPLETED — post-MVP intake channels, template, classification, and triage log |
| P18-002 | COMPLETED — SMTPUTF8, `AbortSignal`, and platform-expansion candidate matrix |
| P18-003 | COMPLETED — connection pooling research; reuse deferred, no queue |
| P18-004 | COMPLETED — compatibility and deprecation policy |
| P18-005 | COMPLETED — PHASE-18 approved 2026-11-18; planning program closed |
| Next step | Releases through `1.2.0` are tagged (`v0.1.0-rc.1` … `v1.2.0`); post-MVP work starts from the [next-cycle backlog](planning/next-cycle-backlog.md) with its own ADR |

## 5. Conventions

- Constraint IDs: C-01–C-40.
- Functional requirements: FR-001–FR-068.
- Non-functional requirements: NFR-001–NFR-014.
- Open questions: OQ-01–OQ-07.
- Phase IDs: P00–P18.
- Task IDs: Pxx-00y.
- Decision IDs: ADR-0001–ADR-0024.
- Risk IDs: R-001–R-049.
- Gate IDs: QG-00–QG-18, QG-03A, QG-A01, QG-A02.
- Milestones: M0–M7.

## 6. Current architecture amendment

Architecture Amendment A01 and ADR-0015 approve Node.js, Bun, and Deno as required MVP runtime targets while preserving exactly two Rust crates and one Node-API native implementation as the default. No Bun/Deno or additional platform compatibility is claimed until Sendust-owned blocking CI passes. Deno requires local node_modules and --allow-ffi. See the [research report](research/multi-runtime-compatibility.md), [ADR-0015](architecture/decisions/0015-multi-runtime-support.md), and [A01 gate](gates/architecture-amendment-a01.md).

## 7. Current architecture amendment A02

A02 adopts a modular monolith inside the existing two-crate workspace. See [ADR-0016](architecture/decisions/0016-modular-monolith.md) and [A02 gate](gates/architecture-amendment-a02.md). It defines core module ownership, SMTP adapter isolation, narrow internal delivery contracts, and architecture enforcement. No new crate, service, plugin framework, or transport is added to MVP.

## 8. Change rule

- Requirement changes update requirements.md and the feature matrix.
- Architecture changes require a new or superseding ADR.
- Task changes update the phase plan, registry, and dependency graph where applicable.
- Completion claims without evidence are invalid.
