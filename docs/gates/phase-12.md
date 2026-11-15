# QG-12 — QA Phase Record

**Decision:** APPROVED — project owner review, 2026-11-15  
**Phase:** P12 — QA  
**Gate:** PHASE-12  
**Prepared:** 2026-11-15  
**Branch:** `main`  
**Evidence:** the local commands in section 6 and CI on the commit that adds this record.

This record is the review package for PHASE-12. The implementation was written by an
agent, so the approval in section 9 is the required human review step, not a
self-approval. Nothing here approves P13 or any later gate.

## 1. Task evidence

| Task | Status | Evidence |
| --- | --- | --- |
| P12-001 | COMPLETED | [`docs/qa/traceability-matrix.md`](../qa/traceability-matrix.md): C-01…C-40, FR-001…FR-068, NFR-001…NFR-014, the limits, lifecycle and error contracts, and AB-01…AB-14 each mapped to a named test or approving gate with its pass condition. All 110 cited identifiers were verified to exist in the repository. |
| P12-002 | COMPLETED | [`docs/qa/integration-coverage.md`](../qa/integration-coverage.md): suite inventory, module-to-suite map, critical-branch table, a Node-free core run (`NO-NODE`, 238 passed), binding parity, skip inventory, and named gaps. |
| P12-003 | COMPLETED | [`docs/qa/typescript-contract-report.md`](../qa/typescript-contract-report.md): 44 facade tests by category, compile-time contract assertions, every status and error code mapped, BUSY one-connection assertion, lifecycle matrix, exact runtime pins. |
| P12-004 | COMPLETED | [`docs/qa/fixture-audit.md`](../qa/fixture-audit.md): the full fixture checklist, `tests/runtime/gc.mjs` closing R-05, four facade sleeps replaced with a connection gate, and two consecutive runs at 238/238, 44/44, 5/5. |
| P12-005 | COMPLETED | This record. |

## 2. Acceptance criteria

| Phase-12 criterion | Result | Evidence |
| --- | --- | --- |
| Every mandatory requirement maps to a passing test or accepted manual evidence | PASS | The matrix has no unowned row; the only deferrals are owned by P13/P14/P15/P16/P17 with their own gates |
| Core, binding, and TypeScript suites pass with exact semantic assertions | PASS | 238 Rust tests across eight suites, 44 facade tests; command traces, statuses, and codes asserted exactly |
| Runtime results are recorded honestly | PASS | Node 22/24/26 run the full facade suite; Bun and Deno run the runtime-neutral smoke entry point by baseline rule; full cross-runtime behaviour is P14-004's gate |
| Fixture catalog is complete and deterministic; repeated runs are stable | PASS | Checklist complete; two consecutive full runs identical; no sequencing sleeps remain in the facade suite |
| No high-severity defect or contract deviation remains | PASS | Section 8 |
| ADR-0016 module ownership, dependency graph, and isolation are verified | PASS | `check-architecture` enforces the layer table, forbidden edges, cycle freedom, SMTP adapter containment, and runtime-neutral TypeScript; every module carries focused unit tests |

## 3. Suite summary

| Suite | Tests | Result |
| --- | --- | --- |
| Core unit (`sendust-core/src`) | 177 | Pass |
| SMTP fault matrix | 26 | Pass |
| Harness self-test | 9 | Pass |
| SMTP client | 6 | Pass |
| SMTP session | 6 | Pass |
| MIME bodies | 6 | Pass |
| MIME conformance | 4 | Pass |
| Leak probes | 4 | Pass |
| Facade (`ts/test`) | 44 | Pass |
| Forced-GC probe | 5 checks | Pass |
| **Total** | **282 + 5 checks** | **Pass** |

Two consecutive runs produced identical results: 238 Rust passed, 44 facade passed, 5 GC checks passed, zero failures.

## 4. Traceability summary

- 40 constraints, 68 functional requirements, 14 non-functional requirements, 5 limits, 5 lifecycle rules, 3 error contracts, and 14 abuse cases — **136 requirement rows**, **0 unowned**.
- Six gaps are listed with an owning phase; each is a deferral to a later gate, not a missing requirement.
- The matrix's identifiers were checked mechanically against the source tree: 110 checked, 0 missing.

## 5. Runtime results

| Runtime | Pin | Executed | Result |
| --- | --- | --- | --- |
| Node.js | 22.23.5 | Typecheck, lint, format, full facade suite, runtime smoke, forced-GC probe | Pass in CI |
| Node.js | 24.22.3 | Full facade suite | Pass in CI |
| Node.js | 26.13.1 | Full facade suite | Pass in CI |
| Bun | 1.4.4 | Runtime-neutral smoke | Pass in CI |
| Deno | 2.9.8 | Runtime-neutral smoke (`--allow-ffi --allow-read --allow-net`) | Pass in CI |

No runtime is claimed *supported* here: support is the tested runtime × OS × CPU × ABI intersection, which P14-004 owns.

## 6. Executed evidence

Run on macOS with the pinned toolchain on 2026-11-15:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked        (twice)
node scripts/check-architecture.mjs
node scripts/check-docs.mjs
node scripts/check-secrets.mjs --selftest
node scripts/check-secrets.mjs
npm run build:native
npm --workspace ./ts run typecheck
npm --workspace ./ts run lint
npm --workspace ./ts run format:check
npm --workspace ./ts run test                        (twice)
node --expose-gc tests/runtime/gc.mjs                (twice)
```

CI on the tip of `main` runs the same set on Linux, including the forced-GC probe in the native binding job.

## 7. Known gaps and owner

| Gap | Owner |
| --- | --- |
| No JS end-to-end `accepted`/`rejected`/`unknown` fixture (no trust anchor in the public API by design) | P14-004 |
| No line/branch coverage percentage (no coverage tool pinned) | Project owner: add one before release reporting |
| Runtime suites are not typechecked (`@types/node` is not a pinned dev dependency) | Project owner |
| Bun and Deno run the smoke entry point only | P14-004 |
| Licence declaration and advisory audit stage in CI | Project owner, before P13/P14 packaging |

## 8. Findings

| Severity | Finding | State |
| --- | --- | --- |
| Low | Fixture gap: no automated forced-GC probe | FIXED in P12-004 |
| Low | Facade tests sequenced on 50–100 ms sleeps | FIXED in P12-004 (event-driven connection gate) |
| Low | No JS end-to-end status fixture | DEFERRED to P14-004 (R-04) |
| Note | Traceability and coverage gaps for tooling | OWNER DECISION, recorded |

No high-severity defect was found in either run, and no contract deviation was observed.

## 9. Decision

**APPROVED, 2026-11-15.** The project owner reviewed this record and its evidence
and approved the PHASE-12 gate. The implementation was written by an agent, so this
approval is the required human review step, not a self-approval. The reviewer
confirmed the three points the record raised:

1. The traceability matrix has no unowned mandatory requirement, and the six
   deferrals are assigned to the right phases.
2. Two consecutive full runs at 238 Rust, 44 facade, and 5 GC checks are
   sufficient stability evidence for this gate.
3. The owner decisions in section 7 (licence, advisory stage, `@types/node`,
   coverage tool) may stay open until release packaging.

P12-001 to P12-005 are COMPLETED and PHASE-12 is APPROVED. Phase 13 becomes
eligible and owns the performance budgets, benchmarks, and regression gate that
NFR-005 depends on.

Approval covers P12 only. It does not approve P13 or any later gate; a failed
compile, test, checker, or runtime result on `main` reopens this gate.

## 10. Residual risks and deferrals

| ID | Item | Decision |
| --- | --- | --- |
| R-04 | No JS fake-TLS relay / end-to-end status fixture | Owned by P14-004; mapping and core semantics are covered now |
| R-05 | No automated forced-GC test | Closed by P12-004 |
| R-06 | Untested big-endian, musl, Windows-on-ARM targets | Owned by P14-003; no support claim is made |
| — | No performance budget | P13-001 defines it; NFR-005 stays open by design |
| — | No packaging or install verification yet | P14; NFR-006/FR-061–FR-063 stay open by design |

Non-goals for this gate: performance benchmarking, packaging, publication,
documentation rewrite, and any new product behaviour.
