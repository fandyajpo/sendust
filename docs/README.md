# Documentation Index

**Sendust — Standalone Native Email Delivery Engine for Node.js**  
Planning baseline: 2026-11-13.

This documentation set is the source of truth for product scope, requirements, approved architecture design, decisions, and execution planning. QG-00, QG-01, and QG-02 are approved. Application implementation, compilation, runtime validation, and publication have not occurred.

## 1. Start here

| Order | Document | Why |
| --- | --- | --- |
| 1 | [Product vision](product/vision.md) | Problem, users, principles, positioning |
| 2 | [Requirements](product/requirements.md) | Constraints and functional/non-functional requirements |
| 3 | [MVP scope](product/mvp-scope.md) | Included, excluded, and success conditions |
| 4 | [Discovery brief](discovery/brief.md) | Falsifiable problem statement and product boundary |
| 5 | [Constraint ledger](discovery/constraint-ledger.md) | C-01..C-30 traceability |
| 6 | [Threat model](discovery/threat-model.md) | Assets, trust boundaries, and abuse cases |
| 7 | [lettre source validation](research/lettre-0.11.23-validation.md) | Pinned API and feature research |
| 8 | [Architecture overview](architecture/overview.md) | Approved architecture shape and boundaries |
| 9 | [System design](architecture/system-design.md) | Modules, types, state machine, test seams |
| 10 | [Public API contract](architecture/public-api.md) | Approved design contract; implementation gate P09 |
| 11 | [Data flow](architecture/data-flow.md) | Send and close across layers |
| 12 | [Error model](architecture/error-model.md) | Outcomes, error codes, redaction |
| 13 | [Architecture approval matrix](architecture/approval-matrix.md) | Approved decisions and downstream validations |
| 14 | [ADR index](architecture/decisions/README.md) | Architecture decision records |
| 15 | [Master roadmap](planning/master-roadmap.md) | All 19 phases and outcomes |
| 16 | [Phase index](planning/phase-index.md) | Entry point to every phase plan |
| 17 | [Task registry](planning/task-registry.md) | All 95 tasks and prerequisites |
| 18 | [Dependency graph](planning/dependency-graph.md) | Phase DAG and critical path |
| 19 | [Milestones](planning/milestones.md) | M0–M7 exit criteria |
| 20 | [Quality gates](planning/quality-gates.md) | QG-00–QG-17 |
| 21 | [Definition of done](planning/definition-of-done.md) | Evidence rules |
| 22 | [Risk registry](planning/risk-registry.md) | Risk IDs, mitigations, owners |
| 23 | [Dependency research](engineering/dependency-research.md) | Observed facts and sources |
| 24 | [Testing strategy](engineering/testing-strategy.md) | Test layers, fixtures, fault matrix |
| 25 | [Security strategy](engineering/security-strategy.md) | TLS, injection, redaction, supply chain |
| 26 | [Reliability strategy](engineering/reliability-strategy.md) | Lifecycle, timeouts, races, cleanup |
| 27 | [Performance strategy](engineering/performance-strategy.md) | Budgets and measurement |
| 28 | [Distribution strategy](engineering/distribution-strategy.md) | Platform matrix and native artifacts |
| 29 | [Release strategy](engineering/release-strategy.md) | Versioning, RC, publication, rollback |
| 30 | [QG-00 discovery gate](gates/phase-00-discovery.md) | Discovery approval evidence |
| 31 | [QG-01 research gate](gates/phase-01-research.md) | Research approval evidence |
| 32 | [QG-02 architecture gate](gates/phase-02-architecture-approval.md) | Architecture approval evidence |

## 2. Directory map

    docs/
    ├── README.md
    ├── discovery/
    │   ├── brief.md
    │   ├── constraint-ledger.md
    │   └── threat-model.md
    ├── research/
    │   └── lettre-0.11.23-validation.md
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
    └── decisions/  (pointer to architecture/decisions)

## 3. Status model

P00–P02 are COMPLETED with approved evidence. P03–P18 remain NOT STARTED. A task is COMPLETED only with evidence per the [definition of done](planning/definition-of-done.md). A phase is COMPLETED only when its gate is approved.

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
| Repository | Documentation and architecture planning |
| Application implementation | Not present in audited snapshot |
| Rust toolchain in audit environment | Not installed; no cargo build executed |
| QG-00 Discovery | APPROVED |
| QG-01 Research | APPROVED with implementation validation gates |
| QG-02 Architecture | APPROVED at design level |
| P03–P18 | NOT STARTED |
| Next task | P03-001 — create workspace and two-crate skeleton |

## 5. Conventions

- Constraint IDs: C-01–C-30.
- Functional requirements: FR-001–FR-068.
- Non-functional requirements: NFR-001–NFR-014.
- Open questions: OQ-01–OQ-07.
- Phase IDs: P00–P18.
- Task IDs: Pxx-00y.
- Decision IDs: ADR-0001–ADR-0014.
- Risk IDs: R-001–R-040.
- Gate IDs: QG-00–QG-17.
- Milestones: M0–M7.

## 6. Change rule

- Requirement changes update requirements.md and the feature matrix.
- Architecture changes require a new or superseding ADR.
- Task changes update the phase plan, registry, and dependency graph where applicable.
- Completion claims without evidence are invalid.
