# Documentation Index

**Sendust — Standalone Native Email Delivery Engine for Node.js, Bun, and Deno**  
Planning baseline: 2026-11-13.

This documentation set is the source of truth for product scope, requirements, approved architecture design, decisions, and execution planning. Repository-wide AI/contributor behavior is governed by [AGENTS.md](../AGENTS.md) and the [AI implementation contract](engineering/ai-implementation-contract.md). QG-00, QG-01, and QG-02 are approved. The P03 repository skeleton, dependency policy, and quality-command scripts exist; application behavior, blocking CI evidence, runtime validation, and publication have not occurred.

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
| 39 | [Dependency and license policy](engineering/dependency-policy.md) | Pinned dependencies, feature admission, feature prohibitions, lockfile rules, and license review |
| 40 | [Test harness contract](engineering/test-harness-contract.md) | Fake SMTP/TLS seams, safety rules, and the fixture catalog |
| 41 | [QG-03 repository baseline record](gates/phase-03-repository-baseline.md) | P03 gate checklist, evidence, and approved decision |

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
    │   ├── ai-implementation-contract.md
    │   └── toolchain-baseline.md
    └── decisions/  (pointer to architecture/decisions)

## 3. Status model

P00–P03 are COMPLETED with approved evidence. P04–P18 remain NOT STARTED. A task is COMPLETED only with evidence per the [definition of done](planning/definition-of-done.md). A phase is COMPLETED only when its gate is approved.

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
| Repository | Documentation, architecture planning, and the P03 workspace skeleton |
| Application implementation | Skeleton only; no product behavior implemented |
| Toolchain validation | Rust 1.100.0 and the Node.js 22.23.5/24.22.3/26.13.1 jobs pass in Sendust CI on branch `feat/p03-repository-skeleton` |
| QG-00 Discovery | APPROVED |
| QG-01 Research | APPROVED with implementation validation gates |
| QG-02 Architecture | APPROVED at design level |
| P03-001–P03-005 | COMPLETED |
| P04–P18 | NOT STARTED |
| Next task | P04-001 — implement core error taxonomy and deterministic validation order |

## 5. Conventions

- Constraint IDs: C-01–C-40.
- Functional requirements: FR-001–FR-068.
- Non-functional requirements: NFR-001–NFR-014.
- Open questions: OQ-01–OQ-07.
- Phase IDs: P00–P18.
- Task IDs: Pxx-00y.
- Decision IDs: ADR-0001–ADR-0016.
- Risk IDs: R-001–R-047.
- Gate IDs: QG-00–QG-17, QG-03A, QG-A01, QG-A02.
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
