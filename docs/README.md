# Documentation Index

**Sendust** — Standalone Native Email Delivery Engine for Node.js.  
Planning baseline: **2026-11-13**. The repository contains planning and architecture documentation; implementation and release status are tracked separately.

This documentation set is the source of truth for product scope, requirements, approved architecture design, decisions, and execution planning. QG-00, QG-01, and QG-02 are approved; implementation, compilation, runtime validation, and publication have not occurred.

## 1. Start here

| Order | Document | Why |
| --- | --- | --- |
| 1 | [Product vision](product/vision.md) | Problem, users, principles, positioning |
| 2 | [Requirements](product/requirements.md) | Constraints, functional requirements, non-functional requirements |
| 3 | [MVP scope](product/mvp-scope.md) | Included, excluded, success conditions |
| 4 | [Feature matrix](planning/feature-matrix.md) | REQUIRED / OPTIONAL / POST-MVP / OUT OF SCOPE with task mapping |
| 5 | [Master roadmap](planning/master-roadmap.md) | All 19 phases (P00–P18), their outcomes, and gates |
| 6 | [Architecture overview](architecture/overview.md) | Approved shape and boundaries |
| 7 | [System design](architecture/system-design.md) | Modules, types, state machine, test seams |
| 8 | [Public API contract](architecture/public-api.md) | Approved design contract; implementation gate P09 |
| 9 | [Data flow](architecture/data-flow.md) | Send and close across layers |
| 10 | [Error model](architecture/error-model.md) | Outcomes, error codes, redaction |
| 11 | [Phase index](planning/phase-index.md) | Entry point to every phase plan |
| 12 | [Task registry](planning/task-registry.md) | All 95 tasks with prerequisites |
| 13 | [Dependency graph](planning/dependency-graph.md) | Phase DAG, critical path, parallel tracks |
| 14 | [Milestones](planning/milestones.md) | M0–M7 with exit criteria |
| 15 | [Quality gates](planning/quality-gates.md) | QG-00–QG-17 |
| 16 | [Definition of done](planning/definition-of-done.md) | Evidence rules |
| 17 | [Risk registry](planning/risk-registry.md) | Risk IDs, mitigations, owners |
| 18 | [Dependency research](engineering/dependency-research.md) | Observed facts and sources, dated 2026-11-13 |
| 19 | [Discovery brief](discovery/brief.md) | Falsifiable problem statement and product boundary |
| 20 | [Constraint ledger](discovery/constraint-ledger.md) | C-01..C-30 traceability |
| 21 | [Threat model](discovery/threat-model.md) | Assets, trust boundaries, and abuse cases |
| 22 | [lettre source validation](research/lettre-0.11.23-validation.md) | Pinned API and feature research |
| 23 | [Architecture approval matrix](architecture/approval-matrix.md) | Approved design decisions and downstream validation |
| 24 | [QG-00 discovery gate](gates/phase-00-discovery.md) | Discovery approval evidence |
| 25 | [QG-01 research gate](gates/phase-01-research.md) | Research approval evidence |
| 26 | [QG-02 architecture gate](gates/phase-02-architecture-approval.md) | Architecture approval evidence |
| 19 | [Testing strategy](engineering/testing-strategy.md) | Test layers, fixtures, fault matrix, flake policy |
| 20 | [Security strategy](engineering/security-strategy.md) | Threat model, TLS, injection, redaction, supply chain |
| 21 | [Reliability strategy](engineering/reliability-strategy.md) | Lifecycle, timeouts, races, cleanup |
| 22 | [Performance strategy](engineering/performance-strategy.md) | Budgets, measurement, optimization order |
| 23 | [Distribution strategy](engineering/distribution-strategy.md) | Platform matrix, artifacts, clean installs |
| 24 | [Release strategy](engineering/release-strategy.md) | Versioning, RC process, publication, rollback |

## 2. Directory map

```text
docs/
├── README.md
├── discovery/\n│   ├── brief.md\n│   ├── constraint-ledger.md\n│   └── threat-model.md\n├── research/\n│   └── lettre-0.11.23-validation.md\n├── gates/\n│   ├── phase-00-discovery.md\n│   ├── phase-01-research.md\n│   └── phase-02-architecture-approval.md\n├── product/
│   ├── vision.md
│   ├── requirements.md
│   └── mvp-scope.md
├── architecture/
│   ├── overview.md
│   ├── system-design.md
│   ├── public-api.md
│   ├── data-flow.md
│   ├── error-model.md
│   └── decisions/
│       ├── README.md
│       └── 0001..0014-*.md
├── planning/
│   ├── master-roadmap.md
│   ├── phase-index.md
│   ├── task-registry.md
│   ├── dependency-graph.md
│   ├── milestones.md
│   ├── risk-registry.md
│   ├── quality-gates.md
│   ├── feature-matrix.md
│   └── definition-of-done.md
├── phases/
│   └── phase-00.md ... phase-18.md
├── engineering/
│   ├── dependency-research.md
│   ├── testing-strategy.md
│   ├── security-strategy.md
│   ├── reliability-strategy.md
│   ├── performance-strategy.md
│   ├── distribution-strategy.md
│   └── release-strategy.md
└── decisions/
    └── README.md          # pointer to architecture/decisions (single ADR home)
```

### Structural notes

- ADRs live only in `architecture/decisions/`. `docs/decisions/README.md` is a pointer; there is no duplicate ADR set.
- Task-level detail lives only in `docs/phases/`. Registries index tasks and never restate acceptance criteria.
- Requirements are authoritative in `product/requirements.md`. Feature classification is authoritative in `planning/feature-matrix.md`.
- [dependency-research.md](engineering/dependency-research.md) records observed facts and their sources; it does not replace the P01 validation tasks.

## 3. Status model

P00–P02 are `COMPLETED` with approved evidence. P03–P18 remain `NOT STARTED`. Allowed states: `NOT STARTED`, `IN PROGRESS`, `BLOCKED`, `IN REVIEW`, `COMPLETED`. A task is `COMPLETED` only with evidence per [definition-of-done.md](planning/definition-of-done.md). A phase is `COMPLETED` only when its gate is approved.

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
| Repository | Documentation repository with committed planning artifacts |
| Application implementation | Not present in the audited repository snapshot |
| Dependencies installed | None |
| Phase 00 | COMPLETED — QG-00 approved |
| Phase 01 | COMPLETED — QG-01 approved |\n| Phase 02 | COMPLETED — QG-02 approved at design level |\n| Phase 18 | NOT STARTED |
| Recommended next task | **P03-001**, repository workspace baseline |

## 5. Conventions

- Constraint IDs: `C-01`–`C-30`.
- Functional requirements: `FR-001`–`FR-068`.
- Non-functional requirements: `NFR-001`–`NFR-014`.
- Open questions: `OQ-01`–`OQ-07`.
- Phase IDs: `P00`–`P18`.
- Task IDs: `Pxx-00y` (for example `P07-003`).
- Decision IDs: `ADR-0001`–`ADR-0014`.
- Risk IDs: `R-001`–`R-040`.
- Gate IDs: `QG-00`–`QG-17`.
- Milestones: `M0`–`M7`.
- Feature IDs: `F-SMTP-xx`, `F-MSG-xx`, `F-ATT-xx`, `F-DEL-xx`, `F-DX-xx`, `F-SEC-xx`, `F-DIS-xx`, `F-OPS-xx`.
- Effort: `S`, `M`, `L`, `XL` (relative, no dates).

## 6. Change rule

- Requirement changes update [requirements.md](product/requirements.md) and the feature matrix.
- Architecture changes require a new or superseding ADR.
- Task changes update the phase plan, the registry, and, if needed, the dependency graph.
- Completion claims without evidence are invalid.
