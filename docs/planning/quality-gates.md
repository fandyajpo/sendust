# Quality Gates

Quality gates are mandatory, evidence-based checkpoints. A gate is `APPROVED` only when every required check has linked evidence and no check is `FAIL` or `BLOCKED`. Gates are cumulative: a later gate assumes all earlier gates still pass on the current commit.

## Gate index

| Gate | Name | Phase | Blocks |
| --- | --- | --- | --- |
| QG-00 | Discovery integrity | P00 | P01 |
| QG-01 | Research evidence | P01 | P02 |
| QG-02 | Architecture approval (design baseline) | P02 | P03 |
| QG-03 | Repository baseline | P03 | P04 |
| QG-04 | Rust safety and core tests | P04 | P05 |
| QG-05 | Node interoperability | P05 | P06, P09 |
| QG-06 | SMTP correctness | P06 | P09 |
| QG-07 | MIME correctness | P07 | P08 |
| QG-08 | Attachment safety | P08 | P09 |
| QG-09 | Public API contract | P09 | P10 |
| QG-10 | Reliability | P10 | P11 |
| QG-11 | Security | P11 | P12 |
| QG-12 | QA completeness | P12 | P13, P14 final |
| QG-13 | Performance | P13 | P14 final, P16 |
| QG-14 | Distribution | P14 | P16 |
| QG-15 | Documentation | P15 | P16 |
| QG-16 | Release engineering | P16 | P17 |
| QG-17 | Release approval | P17 | MVP release |
| QG-A01 | Multi-runtime architecture amendment | A01 | P03 |

## QG-A01 — Multi-runtime architecture amendment

| Check | Requirement | Evidence |
| --- | --- | --- |
| Research | Official Node.js, Bun, Deno, and NAPI-RS compatibility facts recorded | [Multi-runtime research](../research/multi-runtime-compatibility.md) |
| Architecture | Two-crate core and shared Node-API adapter retained; no speculative adapters/WASM | ADR-0015 |
| Runtime policy | Node.js, Bun, and Deno are required MVP targets with blocking Sendust CI | Requirements C-31, feature matrix F-DIS-14..F-DIS-16 |
| Deno setup | Local node_modules and --allow-ffi requirements documented | Research + P03/P14 tasks |
| Platform policy | Explicit runtime × OS × CPU × ABI matrix and evidence rule | ADR-0015, distribution strategy |
| Traceability | Requirements, phases, feature matrix, risks, and gates reconciled | A01 gate record |

A01 approves the design target only. It does not approve runtime or platform compatibility; those remain P05/P14 release blockers.

## QG-00 — Discovery integrity

| Check | Requirement | Evidence |
| --- | --- | --- |
| Problem statement | One falsifiable MVP problem statement with personas and non-goals | P00-001 brief |
| Constraint ledger | Every constraint C-01..C-30 has an owner task and validation path | P00-002 ledger |
| Provenance | Version claims labeled observed or candidate, with sources | P00-003 table |
| Threat boundary | Assets, trust boundaries, abuse cases, test-environment policy | P00-004 record |
| Handoff | ADR decisions and named downstream validation gates | [QG-00 gate record](../gates/phase-00-discovery.md) |

## QG-01 — Research evidence

| Check | Requirement | Evidence |
| --- | --- | --- |
| lettre API | Chosen async API documented with pinned-source citations and per-recipient visibility | [P01 source report](../research/lettre-0.11.23-validation.md) |
| TLS features | Exact feature graph source-reviewed; compile and certificate fixtures remain implementation gates | [P01 source report](../research/lettre-0.11.23-validation.md) |
| Recipient/timeouts | Source-observed RCPT API; coarse-stage table; explicit ADR decisions | [P01 source report](../research/lettre-0.11.23-validation.md) |
| Node lifecycle | Node 22/24/26 candidate-only matrix; explicit close/GC/exit probe plan | [Node runtime plan](../research/node-runtime-validation-plan.md) |
| Decision record | Every mandatory design question selected; implementation proofs have named gates | [QG-01 gate record](../gates/phase-01-research.md) |

## QG-02 — Architecture approval (design baseline)

| Check | Requirement | Evidence |
| --- | --- | --- |
| Crate boundary | Exactly two crates; core has no Node linkage | [Approval matrix](../architecture/approval-matrix.md) |
| Status contract | Positive final DATA=accepted; explicit negative final reply=rejected; indeterminate disposition=unknown; recipient RCPT outcomes preserved | [Approval matrix](../architecture/approval-matrix.md) |
| Error taxonomy | Stable codes/categories; no message parsing required | P02-002 / error model |
| Lifecycle | `open -> closing -> closed`; BUSY; bounded close; no GC | [Approval matrix](../architecture/approval-matrix.md) |
| Timeouts | Connect ≤10s, operation ≤30s, total ≤120s, close ≤125s; lower-only; connect/operation ≤ total ≤ close | [Approval matrix](../architecture/approval-matrix.md) |
| Ownership | Synchronous owned snapshot before async scheduling; no borrowed JS memory | [Approval matrix](../architecture/approval-matrix.md) |
| Limits | 100 recipients, 10/20/30 MiB caps; deterministic precedence | P02-004 |
| ADRs | All P02-scoped ADRs accepted; future-phase ADRs retain explicit downstream gate | [ADR index](../architecture/decisions/README.md) |
| Traceability | C-01..C-36 map to ADRs, implementation tasks, and test gates | [Architecture approval matrix](../architecture/approval-matrix.md) |

## QG-03 — Repository baseline

| Check | Requirement | Evidence |
| --- | --- | --- |
| Workspace | Two crates build; TypeScript workspace builds | CI log |
| Toolchain | `rust-toolchain.toml` and exact Node.js, Bun, and Deno CI versions pinned | Repository files |
| Format/lint | `cargo fmt --check`, `cargo clippy -D warnings`, TS lint/typecheck pass | CI log |
| Tests | Core unit tests and empty integration harness run | CI log |
| Dependency policy | Versions pinned; licenses recorded; advisories checked | Policy + inventory |
| Test seams | Fake SMTP/TLS harness starts, scripts responses, shuts down deterministically | Harness test |
| No placeholders | No `todo!()`, `unimplemented!()`, or stub modules claimed as complete | Search report |

## QG-04 — Rust safety and core tests

| Check | Requirement | Evidence |
| --- | --- | --- |
| Error taxonomy | Every code/category defined; deterministic precedence | Unit tests |
| Address policy | ASCII local part, IDNA domain, Unicode display; CRLF rejected | Tests |
| Message model | Types represent only valid states; caps enforced | Tests |
| State machine | Transitions exhaustive; BUSY and CLOSED tested | Tests |
| Timeouts | Injected clock; finite values only | Tests |
| Core isolation | `cargo tree -p sendust-core` has no `napi` | CI check |
| Panic policy | Recoverable failures are `Result`; no abort strategy | Manifest + review |
| Unsafe | No new `unsafe` without justification | Review record |

## QG-05 — Node interoperability

| Check | Requirement | Evidence |
| --- | --- | --- |
| Node-API level | Compiled level matches declaration | Manifest + probe |
| Buffer copy | Mutation/detachment after call does not affect sent bytes | JS tests |
| Async | `send`/`close` return Promises; rejections carry stable codes | JS tests |
| Lifecycle | Close during connect/DATA; double close; send after close; BUSY | JS tests |
| GC | Forced GC does not close or corrupt a transport | Forced-GC test |
| Process exit | No hang without close; no GC-dependent shutdown | Exit tests |
| Runtime | No custom Tokio runtime created | Code review |
| Runtime matrix | Exact Node.js, Bun, and Deno versions each pass blocking addon, async, lifecycle, and SMTP tests | CI matrix |
| Types | Generated declarations compile in strict mode | Type test |

## QG-05 runtime support rule

Node.js, Bun, and Deno are required MVP targets. A pass on Node.js does not satisfy Bun or Deno. Each runtime must pass the same binding/lifecycle contract on exact pinned versions. Deno tests must use local node_modules and --allow-ffi.

## QG-05 runtime support rule

Node.js, Bun, and Deno are required MVP targets. A pass on Node.js does not satisfy Bun or Deno. Each runtime must pass the same binding/lifecycle contract on exact pinned versions. Deno tests must use local node_modules and --allow-ffi.

## QG-06 — SMTP correctness

| Check | Requirement | Evidence |
| --- | --- | --- |
| Success | Positive final DATA → `accepted` | Command trace |
| Envelope refusal | MAIL refusal or all RCPT replies negative → overall `rejected`; a single RCPT refusal is recipient-level only | Trace |
| Mixed RCPT | One DATA transaction to accepted subset; no DATA when all RCPT replies reject | Trace + transaction count |
| RCPT 421 | Session terminates; no DATA even if an earlier RCPT was accepted | Trace + transaction count |
| Final DATA response | Explicit 4xx/5xx (including `451`) → `rejected`; missing final reply after terminator may have reached relay → `unknown` | SMTP trace + transaction count |
| Auth failure | `535` → `AUTH_FAILED`; no message sent | Trace |
| TLS failures | Self-signed, expired, wrong host, incomplete chain, malformed | Negative tests |
| Timeouts | Connect 10 s, operation 30 s, total 120 s, close 125 s | Injected-clock tests |
| No custom SMTP | No hand-written command strings | Search report |
| No retry/pool | Exactly one transaction; no second connection | Transaction count |
| No per-recipient send | Forbidden path absent | Code + test audit |

## QG-07 — MIME correctness

| Check | Requirement | Evidence |
| --- | --- | --- |
| Structures | text, html, alternative, mixed with attachments | Golden fixtures |
| Headers | RFC 2047 encoding, legal folding, CRLF | Parser conformance |
| BCC | Absent from headers, present in envelope | Trace |
| Filenames | RFC 2231/2047; control characters rejected | Tests |
| Content type | Explicit or fixed extension map; safe fallback | Tests |
| Wire parity | Accounted size equals serialized size | Test |
| Independent parser | All fixtures parse correctly | `mail-parser` report |
| Determinism | Same input produces identical bytes | Repeat-run hash |

## QG-08 — Attachment safety

| Check | Requirement | Evidence |
| --- | --- | --- |
| Default denial | File access disabled unless opted in | Test |
| File types | Regular only; symlink/FIFO/device/directory/socket rejected | File-type matrix |
| Bounded read | Cap + 1 probe; growth detected | Tests |
| Caps | 10 MiB per file, 20 MiB aggregate, 30 MiB wire | Boundary tests |
| Ownership | Bytes copied; no borrowed JS memory | Ownership audit |
| No URL/stream/glob | Types and code paths absent | API audit |
| Redaction | No absolute path in errors | Scan test |

## QG-09 — Public API contract

| Check | Requirement | Evidence |
| --- | --- | --- |
| Surface | Only MVP exports; no pooling/retry/SMTPUTF8/URL/AbortSignal requirement | API audit |
| Results | `accepted`/`rejected`/`unknown`; no delivery claim | Golden tests |
| Errors | Stable classes and codes; `instanceof` works | Golden tests |
| Redaction | No password/body/attachment/path in serialized errors | Marker scans |
| Lifecycle | Close idempotent and bounded; BUSY immediate; CLOSED after close | JS tests |
| Types | Strict; no `any` in public results/errors | Type tests |
| Examples | Runnable against fake relay | Example log |
| Docs hooks | API docs state semantics, defaults, and limits | Review |

## QG-10 — Reliability

| Check | Requirement | Evidence |
| --- | --- | --- |
| Races | Send/close, close/connect, close/DATA, double close | Race matrix |
| Timeout interaction | Stage vs total vs close budgets; no overrun | Injected-clock tests |
| Cleanup | No leaked tasks, sockets, or file descriptors | Accounting tests |
| Fault injection | Reset, stall, abrupt close, malformed reply at each stage | Fault suite |
| Recovery | Transport remains closed or open consistently after failure | State tests |
| No retry | One transaction per send | Transaction count |

## QG-11 — Security

| Check | Requirement | Evidence |
| --- | --- | --- |
| TLS | Verification mandatory; no bypass path | Source audit + negative tests |
| Auth | Credentials never logged or serialized | Redaction scan |
| Injection | CRLF/control chars rejected in all fields | Injection matrix |
| Attachments | Opt-in, regular files, no URLs | File-type matrix |
| Memory | Copied buffers; no borrowed JS across async | Ownership audit |
| Panic | No unwind across FFI | Panic tests |
| Dependencies | Pinned, licensed, advisory-checked, inventoried | SBOM |
| DoS | Caps and timeouts enforced | Boundary tests |
| Tests | No real relay, no secrets, no external network | Network-denial test |

## QG-12 — QA completeness

| Check | Requirement | Evidence |
| --- | --- | --- |
| Traceability | Every requirement maps to implementation and test tasks | P12-001 matrix |
| Core coverage | Error, validation, MIME, state, SMTP paths covered | Coverage report |
| Binding coverage | Ownership, async, lifecycle, mapping covered | Coverage report |
| TypeScript coverage | Contract, lifecycle, errors, examples covered | Coverage report |
| Fixtures | SMTP, TLS, filesystem, JS-boundary fixtures complete | Fixture inventory |
| Regressions | No open regression without a tracked risk | Triage log |
| Determinism | No flaky tests; no arbitrary sleeps | Repeat-run report |

## QG-13 — Performance

| Check | Requirement | Evidence |
| --- | --- | --- |
| Budgets | Defined and approved before measurement | P13-001 |
| Send path | Within budget | Benchmark report |
| MIME | Within budget; linear scaling | Benchmark report |
| Attachments | Peak memory bounded and documented | Measurement report |
| Lifecycle | BUSY/close/timeout overhead acceptable | Measurement report |
| Regression | No unexplained regression | P13-005 |
| Correctness | QA suite still passes | Re-run log |

## QG-14 — Distribution

| Check | Requirement | Evidence |
| --- | --- | --- |
| Matrix | Every declared target built and runtime-tested | Matrix report |
| Clean install | Installs from local artifacts in a clean project | Logs |
| Loader | Correct binary selected; path and version logged | Logs |
| Negatives | Missing/wrong-architecture binary fails clearly | Logs |
| No toolchain | Consumer install needs no Rust compiler | Clean environment |
| No postinstall download | Package review shows optional packages only | Package review |
| ESM/CJS | Both load and share one implementation | Consumer tests |
| Honesty | No untested target claimed | Support table |

## QG-15 — Documentation

| Check | Requirement | Evidence |
| --- | --- | --- |
| Semantics | `accepted` = relay acceptance; no delivery claim | Review |
| Behavior | No retry/pooling/SMTPUTF8/URL claims | Review |
| Matrix | Exact tested Node versions and targets | Review |
| Examples | Runnable and match output | Example log |
| Errors | Every code documented | Catalog review |
| Limits | Caps and defaults documented | Review |
| Links | All internal links resolve | Link checker |
| Maintainer | Build, test, release, and rollback documented | Review |

## QG-16 — Release engineering

| Check | Requirement | Evidence |
| --- | --- | --- |
| Reproducibility | Two clean builds produce identical hashes or documented nondeterminism | Hash comparison |
| Provenance | Source commit, toolchain, dependency inventory, target | Attestation |
| Artifacts | Manifest complete; no missing or extra binaries | Manifest |
| Contents | No tests, source maps, secrets, or unsupported binaries | Tarball inspection |
| Notes | Limitations and semantics accurate | Review |
| Checklist | Every item has owner and evidence | Checklist |
| Rollback | Bad binary, contract bug, security issue, install breakage covered | Plan |
| No publish | Candidate assembly performs no publication | CI log |

## QG-17 — Release approval

| Check | Requirement | Evidence |
| --- | --- | --- |
| RC identity | Version, commit, hashes match the approved candidate | Record |
| Acceptance | MVP feature matrix and all quality gates pass | Review |
| Authorization | Named approver signs the publication decision | Record |
| Publication | Root and platform packages share exact version | Registry metadata |
| Verification | Clean install from registry works on representative targets | Logs |
| Provenance | Registry shows provenance | Registry |
| Closure | Release notes, support matrix, and rollback plan published | Links |

## Enforcement rules

1. A gate is `APPROVED` only with a reviewer-attributed record listing the decision, review date, each check, and its evidence link.
2. A failed check blocks the phase; it is not waived silently.
3. Waivers require a written rationale, owner, deadline, and risk entry. Security gates QG-11 and the TLS checks in QG-06 cannot be waived.
4. Evidence must refer to the exact commit or artifact under review.
5. Later gates re-run earlier mandatory checks; stale evidence is invalid.
6. The final task in each phase is its gate; it cannot be marked complete before the others.
7. A phase cannot be marked `COMPLETED` while any of its gate checks is `FAIL` or `BLOCKED`.

## Gate record template

```text
Gate: QG-XX
Phase: PXX
Commit: <sha>
Artifacts: <hashes or links>
Date: <date>

| Check | Result | Evidence |
| --- | --- | --- |
| ... | PASS/FAIL/N/A | link |

Decision: APPROVED / REJECTED
Waivers: none | <risk id, rationale, owner, deadline>
Reviewer: <name/role>
```
