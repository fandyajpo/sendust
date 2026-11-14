# Master Task Registry

**Authoritative task detail lives in the phase plans.** This registry indexes all 95 phase tasks plus 7 architecture-amendment tasks and their prerequisites so the project can be tracked centrally. It does not restate acceptance criteria or implementation detail.

- Total phases: 19
- Phase tasks: 95
- Architecture amendment tasks: 7 (A01-001–A01-004, A02-001–A02-003)
- Completed tasks: 30 (P00–P05)
- In-review tasks: 0
- In-progress tasks: 0
- Not-started tasks: 65 (P06–P18)
- P00–P02 tasks are `COMPLETED` with approved gate evidence. P03-001–P03-005 are `COMPLETED` with CI evidence and an approved QG-03 record; P04-001 to P04-005 are `COMPLETED` with an approved PHASE-04 record; P05-001 to P05-005 are `COMPLETED` with an approved PHASE-05 record; P06–P18 are `NOT STARTED`. A01-001–A01-004 and A02-001–A02-003 are completed as documentation/design work; runtime and modular implementation validation remains pending.
- Effort is relative: S, M, L, XL. No delivery dates.
- Gate tasks are the final task in each phase and must not be marked complete before their phase evidence exists.

## Architecture Amendment A01 tasks

| Task | Title | Prerequisites | Status |
| --- | --- | --- | --- |
| A01-001 | Record official runtime compatibility evidence | P01-004 | COMPLETED |
| A01-002 | Freeze multi-runtime/platform design | A01-001 | COMPLETED |
| A01-003 | Reconcile requirements, phases, CI, distribution and release docs | A01-001, A01-002 | COMPLETED |
| A01-004 | Amendment gate review | A01-001, A01-002, A01-003 | COMPLETED |

## Architecture Amendment A02 tasks

| Task | Title | Prerequisites | Status |
| --- | --- | --- | --- |
| A02-001 | Define modular-monolith boundaries and module ownership | A01-004, P02-005 | COMPLETED |
| A02-002 | Define SMTP delivery port and adapter isolation policy | A02-001 | COMPLETED |
| A02-003 | Review modular architecture, constraints, and implementation handoff | A02-001, A02-002 | COMPLETED |

## Registry

| Phase | Phase name | Task | Title | Size | Prerequisites | Status |
| --- | --- | --- | --- | --- | --- | --- |
| [P00](../phases/phase-00.md) | Discovery | [P00-001](../phases/phase-00.md) | Capture MVP problem statement, personas, and non-goals | S | None | COMPLETED |
| [P00](../phases/phase-00.md) | Discovery | [P00-002](../phases/phase-00.md) | Build the authoritative constraint ledger and open-question register | M | P00-001 | COMPLETED |
| [P00](../phases/phase-00.md) | Discovery | [P00-003](../phases/phase-00.md) | Record platform and dependency provenance without installing or coding | S | P00-001 | COMPLETED |
| [P00](../phases/phase-00.md) | Discovery | [P00-004](../phases/phase-00.md) | Define the threat, operational, and test-environment boundaries | M | P00-002 | COMPLETED |
| [P00](../phases/phase-00.md) | Discovery | [P00-005](../phases/phase-00.md) | Run the discovery gate and hand off research questions | S | P00-001, P00-002, P00-003, P00-004 | COMPLETED |
| [P01](../phases/phase-01.md) | Research | [P01-001](../phases/phase-01.md) | Analyze lettre 0.11.23 async SMTP and version constraints | M | P00-005 | COMPLETED |
| [P01](../phases/phase-01.md) | Research | [P01-002](../phases/phase-01.md) | Validate rustls ring/webpki-roots feature and verification posture | S | P01-001 | COMPLETED |
| [P01](../phases/phase-01.md) | Research | [P01-003](../phases/phase-01.md) | Determine recipient partial-failure and timeout-stage observability | L | P01-001 | COMPLETED |
| [P01](../phases/phase-01.md) | Research | [P01-004](../phases/phase-01.md) | Research Node-API 8 and define Node runtime/lifecycle probe plan | M | P00-005 | COMPLETED |
| [P01](../phases/phase-01.md) | Research | [P01-005](../phases/phase-01.md) | Research decision gate and ADR handoff | S | P01-001, P01-002, P01-003, P01-004 | COMPLETED |
| [P02](../phases/phase-02.md) | Architecture | [P02-001](../phases/phase-02.md) | Define crate boundaries and dependency direction | M | P01-005 | COMPLETED |
| [P02](../phases/phase-02.md) | Architecture | [P02-002](../phases/phase-02.md) | Specify the public status, result, and error contract | L | P02-001, P01-003 | COMPLETED |
| [P02](../phases/phase-02.md) | Architecture | [P02-003](../phases/phase-02.md) | Design transport lifecycle, concurrency, and timeout budgets | L | P02-001 | COMPLETED |
| [P02](../phases/phase-02.md) | Architecture | [P02-004](../phases/phase-02.md) | Specify memory ownership, limits, and attachment policy | M | P02-001 | COMPLETED |
| [P02](../phases/phase-02.md) | Architecture | [P02-005](../phases/phase-02.md) | Architecture review, ADR freeze, and implementation handoff | S | P02-001, P02-002, P02-003, P02-004 | COMPLETED |
| [P03](../phases/phase-03.md) | Repository | [P03-001](../phases/phase-03.md) | Create the workspace and two-crate skeleton | M | P02-005, A01-004, A02-003 | COMPLETED |
| [P03](../phases/phase-03.md) | Repository | [P03-002](../phases/phase-03.md) | Define dependency pinning, feature, and license policy | S | P03-001, P01-002 | COMPLETED |
| [P03](../phases/phase-03.md) | Repository | [P03-003](../phases/phase-03.md) | Establish formatting, linting, type-check, and unit-test commands | M | P03-001 | COMPLETED |
| [P03](../phases/phase-03.md) | Repository | [P03-004](../phases/phase-03.md) | Scaffold deterministic fake SMTP and TLS test harness seams | L | P03-001 | COMPLETED |
| [P03](../phases/phase-03.md) | Repository | [P03-005](../phases/phase-03.md) | Repository gate and baseline verification | S | P03-001, P03-002, P03-003, P03-004 | COMPLETED |
| [P04](../phases/phase-04.md) | Rust core | [P04-001](../phases/phase-04.md) | Implement core error taxonomy and deterministic validation order | M | P03-005, P02-002 | COMPLETED |
| [P04](../phases/phase-04.md) | Rust core | [P04-002](../phases/phase-04.md) | Implement address, header, and Unicode/IDNA policy | L | P03-005, P02-004 | COMPLETED |
| [P04](../phases/phase-04.md) | Rust core | [P04-003](../phases/phase-04.md) | Implement the message model and hard size accounting | L | P03-005, P02-004 | COMPLETED |
| [P04](../phases/phase-04.md) | Rust core | [P04-004](../phases/phase-04.md) | Implement transport state machine, deadlines, and BUSY semantics | L | P03-005, P02-003 | COMPLETED |
| [P04](../phases/phase-04.md) | Rust core | [P04-005](../phases/phase-04.md) | Implement lettre-backed SMTP orchestration and core phase test gate | XL | P04-001, P04-002, P04-003, P04-004, P01-001, P01-003 | COMPLETED |
| [P05](../phases/phase-05.md) | Binding | [P05-001](../phases/phase-05.md) | Define Node-API 8 binding surface | M | P04-005 | COMPLETED |
| [P05](../phases/phase-05.md) | Binding | [P05-002](../phases/phase-05.md) | Implement owned Buffer and string conversion | M | P05-001 | COMPLETED |
| [P05](../phases/phase-05.md) | Binding | [P05-003](../phases/phase-05.md) | Implement async task and lifecycle adapter | L | P05-001, P04-004 | COMPLETED |
| [P05](../phases/phase-05.md) | Binding | [P05-004](../phases/phase-05.md) | Implement stable result/error conversion | M | P05-002, P05-003 | COMPLETED |
| [P05](../phases/phase-05.md) | Binding | [P05-005](../phases/phase-05.md) | Binding phase gate | S | P05-001, P05-002, P05-003, P05-004 | COMPLETED |
| [P06](../phases/phase-06.md) | SMTP | [P06-001](../phases/phase-06.md) | Implement the SMTP command codec and response parser | L | P04-005 | NOT STARTED |
| [P06](../phases/phase-06.md) | SMTP | [P06-002](../phases/phase-06.md) | Implement the native connection, implicit TLS, and STARTTLS client | L | P06-001 | NOT STARTED |
| [P06](../phases/phase-06.md) | SMTP | [P06-003](../phases/phase-06.md) | Implement SMTP AUTH and the session state machine | L | P06-001, P06-002 | NOT STARTED |
| [P06](../phases/phase-06.md) | SMTP | [P06-004](../phases/phase-06.md) | Swap the delivery adapter and prove the differential fault matrix | XL | P06-003, P04-005 | NOT STARTED |
| [P06](../phases/phase-06.md) | SMTP | [P06-005](../phases/phase-06.md) | Native SMTP engine phase gate | M | P06-001, P06-002, P06-003, P06-004 | NOT STARTED |
| [P07](../phases/phase-07.md) | MIME | [P07-001](../phases/phase-07.md) | Implement header model and safe encoding | L | P04-002, P04-003 | NOT STARTED |
| [P07](../phases/phase-07.md) | MIME | [P07-002](../phases/phase-07.md) | Implement plain text and HTML alternative bodies | M | P07-001 | NOT STARTED |
| [P07](../phases/phase-07.md) | MIME | [P07-003](../phases/phase-07.md) | Implement filename and content-type handling policy | M | P07-001 | NOT STARTED |
| [P07](../phases/phase-07.md) | MIME | [P07-004](../phases/phase-07.md) | Implement deterministic serialization and wire-size parity | L | P07-001, P07-002, P07-003 | NOT STARTED |
| [P07](../phases/phase-07.md) | MIME | [P07-005](../phases/phase-07.md) | MIME conformance and phase gate | M | P07-001, P07-002, P07-003, P07-004 | NOT STARTED |
| [P08](../phases/phase-08.md) | Attachments | [P08-001](../phases/phase-08.md) | Define opt-in filesystem configuration and descriptor contract | M | P07-005, P05-005 | NOT STARTED |
| [P08](../phases/phase-08.md) | Attachments | [P08-002](../phases/phase-08.md) | Implement safe regular-file validation and bounded reads | L | P08-001 | NOT STARTED |
| [P08](../phases/phase-08.md) | Attachments | [P08-003](../phases/phase-08.md) | Enforce per-file, aggregate, and wire caps with attachments | M | P08-002, P04-003 | NOT STARTED |
| [P08](../phases/phase-08.md) | Attachments | [P08-004](../phases/phase-08.md) | Integrate attachment bytes with MIME and copied ownership | L | P08-002, P08-003, P07-004 | NOT STARTED |
| [P08](../phases/phase-08.md) | Attachments | [P08-005](../phases/phase-08.md) | Attachment fault-matrix phase gate | M | P08-001, P08-002, P08-003, P08-004 | NOT STARTED |
| [P09](../phases/phase-09.md) | Public delivery API | [P09-001](../phases/phase-09.md) | Define public TypeScript types and transport lifecycle API | L | P05-005, P06-005 | NOT STARTED |
| [P09](../phases/phase-09.md) | Public delivery API | [P09-002](../phases/phase-09.md) | Implement send composition and result mapping | L | P09-001, P06-005 | NOT STARTED |
| [P09](../phases/phase-09.md) | Public delivery API | [P09-003](../phases/phase-09.md) | Implement public error classes and redaction | M | P09-001 | NOT STARTED |
| [P09](../phases/phase-09.md) | Public delivery API | [P09-004](../phases/phase-09.md) | Implement explicit close and busy behavior at the public boundary | M | P09-001, P09-002 | NOT STARTED |
| [P09](../phases/phase-09.md) | Public delivery API | [P09-005](../phases/phase-09.md) | Public API contract gate and usage examples | S | P09-001, P09-002, P09-003, P09-004 | NOT STARTED |
| [P10](../phases/phase-10.md) | Reliability | [P10-001](../phases/phase-10.md) | Implement lifecycle race hardening | L | P09-005 | NOT STARTED |
| [P10](../phases/phase-10.md) | Reliability | [P10-002](../phases/phase-10.md) | Enforce timeout-budget interaction and bounded close drain | L | P10-001 | NOT STARTED |
| [P10](../phases/phase-10.md) | Reliability | [P10-003](../phases/phase-10.md) | Implement resource cleanup and leak prevention | M | P10-001 | NOT STARTED |
| [P10](../phases/phase-10.md) | Reliability | [P10-004](../phases/phase-10.md) | Build the reliability fault-injection suite | XL | P10-001, P10-002, P10-003 | NOT STARTED |
| [P10](../phases/phase-10.md) | Reliability | [P10-005](../phases/phase-10.md) | Reliability phase gate and failure-mode review | M | P10-001, P10-002, P10-003, P10-004 | NOT STARTED |
| [P11](../phases/phase-11.md) | Security | [P11-001](../phases/phase-11.md) | Execute the threat model and trust-boundary audit | L | P10-005, P00-004 | NOT STARTED |
| [P11](../phases/phase-11.md) | Security | [P11-002](../phases/phase-11.md) | Prove TLS and authentication security posture | M | P11-001, P06-002 | NOT STARTED |
| [P11](../phases/phase-11.md) | Security | [P11-003](../phases/phase-11.md) | Audit validation, MIME, and attachment security | L | P11-001, P07-005, P08-005 | NOT STARTED |
| [P11](../phases/phase-11.md) | Security | [P11-004](../phases/phase-11.md) | Audit memory ownership, redaction, and denial-of-service bounds | M | P11-001, P10-003 | NOT STARTED |
| [P11](../phases/phase-11.md) | Security | [P11-005](../phases/phase-11.md) | Security regression gate and remediation sign-off | M | P11-001, P11-002, P11-003, P11-004 | NOT STARTED |
| [P12](../phases/phase-12.md) | QA | [P12-001](../phases/phase-12.md) | Build the requirements-to-test traceability matrix | M | P11-005 | NOT STARTED |
| [P12](../phases/phase-12.md) | QA | [P12-002](../phases/phase-12.md) | Execute core and binding integration coverage | L | P12-001 | NOT STARTED |
| [P12](../phases/phase-12.md) | QA | [P12-003](../phases/phase-12.md) | Execute TypeScript contract, lifecycle, and error tests | L | P12-001, P09-005 | NOT STARTED |
| [P12](../phases/phase-12.md) | QA | [P12-004](../phases/phase-12.md) | Run fixture completeness and regression triage | M | P12-002, P12-003 | NOT STARTED |
| [P12](../phases/phase-12.md) | QA | [P12-005](../phases/phase-12.md) | QA exit gate and evidence package | S | P12-001, P12-002, P12-003, P12-004 | NOT STARTED |
| [P13](../phases/phase-13.md) | Performance | [P13-001](../phases/phase-13.md) | Define performance budgets and measurement method | M | P12-005 | NOT STARTED |
| [P13](../phases/phase-13.md) | Performance | [P13-002](../phases/phase-13.md) | Benchmark core send path and MIME serialization | L | P13-001 | NOT STARTED |
| [P13](../phases/phase-13.md) | Performance | [P13-003](../phases/phase-13.md) | Benchmark attachment reads, copying, and memory bounds | L | P13-001 | NOT STARTED |
| [P13](../phases/phase-13.md) | Performance | [P13-004](../phases/phase-13.md) | Measure lifecycle, BUSY, timeout, and close overhead | M | P13-001, P10-005 | NOT STARTED |
| [P13](../phases/phase-13.md) | Performance | [P13-005](../phases/phase-13.md) | Performance regression gate and final report | S | P13-001, P13-002, P13-003, P13-004 | NOT STARTED |
| [P14](../phases/phase-14.md) | Distribution | [P14-001](../phases/phase-14.md) | Early distribution smoke from a clean consumer | M | P05-005, P03-005 | NOT STARTED |
| [P14](../phases/phase-14.md) | Distribution | [P14-002](../phases/phase-14.md) | Define package metadata, exports, and native artifact layout | M | P14-001 | NOT STARTED |
| [P14](../phases/phase-14.md) | Distribution | [P14-003](../phases/phase-14.md) | Specify cross-platform build and prebuilt artifact matrix | L | P14-002 | NOT STARTED |
| [P14](../phases/phase-14.md) | Distribution | [P14-004](../phases/phase-14.md) | Validate clean installs and runtime loading across the matrix | L | P14-003 | NOT STARTED |
| [P14](../phases/phase-14.md) | Distribution | [P14-005](../phases/phase-14.md) | Final distribution gate using QA and performance evidence | M | P14-004, P12-005, P13-005 | NOT STARTED |
| [P15](../phases/phase-15.md) | Documentation | [P15-001](../phases/phase-15.md) | Write the quick start and semantic contract | M | P09-005 | NOT STARTED |
| [P15](../phases/phase-15.md) | Documentation | [P15-002](../phases/phase-15.md) | Document lifecycle, timeouts, concurrency, and limits | L | P15-001 | NOT STARTED |
| [P15](../phases/phase-15.md) | Documentation | [P15-003](../phases/phase-15.md) | Document addresses, MIME, and attachments | L | P15-001 | NOT STARTED |
| [P15](../phases/phase-15.md) | Documentation | [P15-004](../phases/phase-15.md) | Write the error catalog and security/operations guide | M | P15-001, P11-005 | NOT STARTED |
| [P15](../phases/phase-15.md) | Documentation | [P15-005](../phases/phase-15.md) | Documentation accuracy gate and maintainer guide | S | P15-001, P15-002, P15-003, P15-004 | NOT STARTED |
| [P16](../phases/phase-16.md) | Release engineering | [P16-001](../phases/phase-16.md) | Define versioning, tagging, and changelog policy | M | P14-005, P15-005 | NOT STARTED |
| [P16](../phases/phase-16.md) | Release engineering | [P16-002](../phases/phase-16.md) | Assemble reproducible build and artifact provenance | L | P16-001 | NOT STARTED |
| [P16](../phases/phase-16.md) | Release engineering | [P16-003](../phases/phase-16.md) | Draft release notes and migration/limitations section | M | P16-001, P15-005 | NOT STARTED |
| [P16](../phases/phase-16.md) | Release engineering | [P16-004](../phases/phase-16.md) | Create the release checklist and rollback plan | M | P16-002, P16-003, P12-005, P13-005 | NOT STARTED |
| [P16](../phases/phase-16.md) | Release engineering | [P16-005](../phases/phase-16.md) | Release-engineering gate and RC handoff | S | P16-001, P16-002, P16-003, P16-004 | NOT STARTED |
| [P17](../phases/phase-17.md) | RC and MVP approval/publication | [P17-001](../phases/phase-17.md) | Verify RC identity, artifacts, and evidence completeness | M | P16-005 | NOT STARTED |
| [P17](../phases/phase-17.md) | RC and MVP approval/publication | [P17-002](../phases/phase-17.md) | Run final MVP acceptance review against authoritative constraints | L | P17-001 | NOT STARTED |
| [P17](../phases/phase-17.md) | RC and MVP approval/publication | [P17-003](../phases/phase-17.md) | Execute the publication checklist and release authorization | M | P17-002 | NOT STARTED |
| [P17](../phases/phase-17.md) | RC and MVP approval/publication | [P17-004](../phases/phase-17.md) | Publish the approved MVP and run post-publication verification | L | P17-003 | NOT STARTED |
| [P17](../phases/phase-17.md) | RC and MVP approval/publication | [P17-005](../phases/phase-17.md) | MVP approval record, publication closure, and rollback readiness | M | P17-001, P17-002, P17-003, P17-004 | NOT STARTED |
| [P18](../phases/phase-18.md) | Post-MVP | [P18-001](../phases/phase-18.md) | Collect post-MVP feedback and defect intake | M | P17-005 | NOT STARTED |
| [P18](../phases/phase-18.md) | Post-MVP | [P18-002](../phases/phase-18.md) | Evaluate SMTPUTF8, AbortSignal, and platform expansion candidates | L | P18-001 | NOT STARTED |
| [P18](../phases/phase-18.md) | Post-MVP | [P18-003](../phases/phase-18.md) | Evaluate connection pooling as separate future research; queue/job orchestration is held outside the roadmap | L | P18-001 | NOT STARTED |
| [P18](../phases/phase-18.md) | Post-MVP | [P18-004](../phases/phase-18.md) | Define the next compatibility and deprecation policy | M | P18-002, P18-003 | NOT STARTED |
| [P18](../phases/phase-18.md) | Post-MVP | [P18-005](../phases/phase-18.md) | Post-MVP intake gate and next-cycle handoff | S | P18-001, P18-002, P18-003, P18-004 | NOT STARTED |

## Task ID ranges

| Phase | Range | Count |
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

## Gate tasks

| Gate | Phase | Evidence required |
| --- | --- | --- |
| PHASE-00 | P00 | [P00-005](../phases/phase-00.md) gate record |
| PHASE-01 | P01 | [P01-005](../phases/phase-01.md) gate record |
| PHASE-02 | P02 | [P02-005](../phases/phase-02.md) gate record |
| PHASE-03 | P03 | [P03-005](../phases/phase-03.md) gate record |
| PHASE-04 | P04 | [P04-005](../phases/phase-04.md) gate record |
| PHASE-05 | P05 | [P05-005](../phases/phase-05.md) gate record |
| PHASE-06 | P06 | [P06-005](../phases/phase-06.md) gate record |
| PHASE-07 | P07 | [P07-005](../phases/phase-07.md) gate record |
| PHASE-08 | P08 | [P08-005](../phases/phase-08.md) gate record |
| PHASE-09 | P09 | [P09-005](../phases/phase-09.md) gate record |
| PHASE-10 | P10 | [P10-005](../phases/phase-10.md) gate record |
| PHASE-11 | P11 | [P11-005](../phases/phase-11.md) gate record |
| PHASE-12 | P12 | [P12-005](../phases/phase-12.md) gate record |
| PHASE-13 | P13 | [P13-005](../phases/phase-13.md) gate record |
| PHASE-14 | P14 | [P14-005](../phases/phase-14.md) gate record |
| PHASE-15 | P15 | [P15-005](../phases/phase-15.md) gate record |
| PHASE-16 | P16 | [P16-005](../phases/phase-16.md) gate record |
| PHASE-17 | P17 | [P17-005](../phases/phase-17.md) gate record |
| PHASE-18 | P18 | [P18-005](../phases/phase-18.md) gate record |

## Status update rule

A task may move to `IN PROGRESS` only when its prerequisites are `COMPLETED` or explicitly waived in a gate record. A task may move to `COMPLETED` only with the evidence named in its phase plan and the [definition of done](definition-of-done.md). Status changes must update both the phase plan and this registry in the same change.
