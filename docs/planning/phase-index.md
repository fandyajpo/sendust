# Phase Index

Entry point to all 19 Sendust phases. P00–P02 are completed with reviewed gate evidence; P03–P18 remain not started. Each phase plan is the authoritative source for task detail, acceptance criteria, and gate conditions. This index routes and summarizes only.

## Phase list

| Phase | Name | Objective | Depends on | Tasks | Gate | Status |
| --- | --- | --- | --- | --- | --- | --- |
| [P00](../phases/phase-00.md) | Discovery | Fix the MVP problem, constraints, semantics, threat boundary, and open questions | — | 5 | P00-005 | COMPLETED |
| [P01](../phases/phase-01.md) | Research | Resolve lettre API, TLS feature design, recipient observability, and Node runtime probe requirements | P00 | 5 | P01-005 | COMPLETED |
| [P02](../phases/phase-02.md) | Architecture | Approve crate boundaries, status/error contract, lifecycle, ownership, and limits | P01 | 5 | P02-005 | COMPLETED |
| [P03](../phases/phase-03.md) | Repository | Create workspace, toolchain, policies, and deterministic test seams | P02 | 5 | P03-005 | NOT STARTED |
| [P04](../phases/phase-04.md) | Rust core | Implement errors, validation, message model, state machine, and SMTP orchestration | P03 | 5 | P04-005 | NOT STARTED |
| [P05](../phases/phase-05.md) | Binding | Expose core through Node-API 8 with owned memory and stable mapping | P04 | 5 | P05-005 | NOT STARTED |
| [P06](../phases/phase-06.md) | SMTP | Relay sessions, TLS/auth, recipient policy, and response normalization | P04, P01 | 5 | P06-005 | NOT STARTED |
| [P07](../phases/phase-07.md) | MIME | Headers, alternative bodies, filenames, and deterministic serialization | P04 | 5 | P07-005 | NOT STARTED |
| [P08](../phases/phase-08.md) | Attachments | Opt-in files, safe reads, caps, and MIME integration | P07, P05 | 5 | P08-005 | NOT STARTED |
| [P09](../phases/phase-09.md) | Public API | TypeScript transport, send, close, errors, and contract gate | P05, P06, P08 | 5 | P09-005 | NOT STARTED |
| [P10](../phases/phase-10.md) | Reliability | Race hardening, timeout interaction, cleanup, and fault injection | P09 | 5 | P10-005 | NOT STARTED |
| [P11](../phases/phase-11.md) | Security | Threat model, TLS/auth proof, validation/MIME/attachment audit, redaction | P10 | 5 | P11-005 | NOT STARTED |
| [P12](../phases/phase-12.md) | QA | Traceability, integration coverage, contract tests, and fixture completeness | P11 | 5 | P12-005 | NOT STARTED |
| [P13](../phases/phase-13.md) | Performance | Budgets, send-path and MIME benchmarks, attachment memory, lifecycle overhead | P12 | 5 | P13-005 | NOT STARTED |
| [P14](../phases/phase-14.md) | Distribution | Early smoke, metadata, platform matrix, clean installs, and final gate | P05, P03 (early); P12/P13 via tasks | 5 | P14-005 | NOT STARTED |
| [P15](../phases/phase-15.md) | Documentation | Quick start, lifecycle, addresses/MIME/attachments, errors/security, accuracy gate | P09 | 5 | P15-005 | NOT STARTED |
| [P16](../phases/phase-16.md) | Release engineering | Versioning, reproducible build, notes, checklist, and RC handoff | P14, P12, P15, P13 | 5 | P16-005 | NOT STARTED |
| [P17](../phases/phase-17.md) | RC and MVP approval | Verify RC, final acceptance, authorize, publish, and close | P16 | 5 | P17-005 | NOT STARTED |
| [P18](../phases/phase-18.md) | Post-MVP | Feedback intake, feature research, compatibility policy, and next cycle | P17 | 5 | P18-005 | NOT STARTED |

## Task ranges

| Phase | Task IDs | Count |
| --- | --- | --- |
| P00 | P00-001–P00-005 | 5 |
| P01 | P01-001–P01-005 | 5 |
| P02 | P02-001–P02-005 | 5 |
| P03 | P03-001–P03-005 | 5 |
| P04 | P04-001–P04-005 | 5 |
| P05 | P05-001–P05-005 | 5 |
| P06 | P06-001–P06-005 | 5 |
| P07 | P07-001–P07-005 | 5 |
| P08 | P08-001–P08-005 | 5 |
| P09 | P09-001–P09-005 | 5 |
| P10 | P10-001–P10-005 | 5 |
| P11 | P11-001–P11-005 | 5 |
| P12 | P12-001–P12-005 | 5 |
| P13 | P13-001–P13-005 | 5 |
| P14 | P14-001–P14-005 | 5 |
| P15 | P15-001–P15-005 | 5 |
| P16 | P16-001–P16-005 | 5 |
| P17 | P17-001–P17-005 | 5 |
| P18 | P18-001–P18-005 | 5 |

## Status summary

| Status | Count |
| --- | --- |
| NOT STARTED | 16 |
| IN PROGRESS | 0 |
| BLOCKED | 0 |
| IN REVIEW | 0 |
| COMPLETED | 3 |

## Parallel work

The phase DAG is mostly sequential because each phase consumes the previous phase’s frozen artifacts. Genuine parallelism exists only where a phase does not consume another’s output:

- **P01-004** (Node/Node-API research and probe planning) can run in parallel with P01-001–P01-003 (lettre/TLS research) after P00-005.
- **P07** (MIME) can run in parallel with **P06** (SMTP) after P04; both consume core types but not each other.
- **P15** (documentation) can begin after P09 and proceed in parallel with P10–P14, but P15-004 requires P11-005 and P15-005 requires all documentation tasks.
- **P14-001–P14-003** can begin after P05-005/P03-005 in parallel with P09–P13; only P14-005 is gated on P12-005 and P13-005.

See [dependency-graph.md](dependency-graph.md) for the graph and critical path.

## Reading a phase plan

Every phase plan has the same 11 sections: objective, scope, tasks, technical implementation details, dependencies, risks and edge cases, testing requirements, acceptance criteria, deliverables, completion gate, and status. Task IDs are stable and must not be renumbered after work begins.
