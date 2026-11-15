# Full Repository Audit — 2026-11-15

**Scope:** the whole repository at the PHASE-12 approval (`fbf5fd4`): every phase plan and gate record, every live architecture and planning document, the product requirements, the QA records, the build and CI configuration, and the implemented crates.
**Method:** read-through of the live documents, a claim-by-claim cross-check against the code and the current test evidence, static checkers (`check-architecture`, `check-docs`, `check-secrets`), and the latest CI run. Historical records were read for what they are — records of the moment they were written.
**Result:** seven low-severity documentation defects, all fixed in this audit; no code defect, no gate-evidence defect, no contract deviation.

## 1. Repository state

| Item | State |
| --- | --- |
| Phase progress | P00–P12 COMPLETED with approved gates; P13 IN PROGRESS (P13-001 in review) |
| Tasks | 65 completed, 1 in review, 29 not started |
| Tests | 238 Rust (8 suites), 44 facade, 5 forced-GC checks |
| Checkers | Architecture, docs, secret scan all PASS |
| CI | 8/8 jobs green on the last recorded run; the current commit's run was still finishing when this audit was written |
| Dependencies | 13 direct, all permissive; exact pins; no SMTP engine in the graph |
| Public package | Not published; P14 owns packaging |

## 2. Findings

| ID | Severity | Finding | Disposition |
| --- | --- | --- | --- |
| F-01 | Low | The root README said "the modular architecture is approved at design level only; module enforcement and runtime behavior remain unvalidated", which P12 disproved (the architecture checker enforces the module graph, and runtime behavior is tested on all pinned runtimes) | FIXED: the sentence now states that boundaries are enforced by static checks and runtime behavior is validated |
| F-02 | Low | The same README separately said "Runtime, platform, and module-boundary validation remain pending" | FIXED: narrowed to platform and distribution validation, which are genuinely pending at P14 |
| F-03 | Low | PHASE-12's technical details named the superseded engine (`TS -> napi -> core -> lettre`, `lettre 0.11.23`) as the frozen architecture | FIXED: names the native SMTP engine |
| F-04 | Low | PHASE-13's technical details carried the same stale engine reference | FIXED |
| F-05 | Low | The architecture approval matrix said "lettre is a dev-only oracle", though the oracle was removed at PHASE-06 | FIXED: records that it was removed at PHASE-06 |
| F-06 | Low | The ADR index summarised ADR-0010 as "use lettre message primitives where correct", which ADR-0010 itself amends ("lettre is gone, so every encoding primitive is core-owned") | FIXED: the index matches the amended ADR |
| F-07 | Low | The milestones document described lettre decisions as current milestones that had been reached | FIXED: annotated as the historical decision, with ADR-0017 noted as the replacement |

No finding required a code change, a test change, or a gate reopening.

## 3. What was verified and holds

- **Architecture.** Exactly two crates, one-way `napi -> core`, ADR-0016 layering, no cycle, no SMTP policy in the binding or the facade, no `lettre` in tree or source. The checker rejects every forbidden marker, including TLS bypass strings and URL schemes.
- **Gates.** P00–P12 records each state their decision, evidence, and residuals; the P11 and P12 records were approved after review, and neither claims more than the evidence shows.
- **Numbers.** Every count in the records (238 Rust, 44 facade, 5 GC, 136 traceability rows, 13 direct dependencies) was re-derived and matched the running system.
- **Security.** No secret shape outside the documented TLS fixture keys; credentials, paths, and bodies are excluded from errors; no logging crate exists.
- **Honesty of deferrals.** R-04 (JS end-to-end status fixture) is owned by P14-004, R-05 (forced GC) is closed by P12-004, and the owner decisions (licence, advisory stage, `@types/node`, coverage tool) are stated as open rather than assumed.
- **Historical records.** The P00–P06 gate records describe lettre as the engine, which is correct for their date; they were left untouched, and only live documents were corrected.

## 4. Open items and owners

| Item | Owner |
| --- | --- |
| Licence declaration and an advisory (`cargo audit`/`npm audit`) CI stage | Project owner, before P13/P14 packaging |
| `@types/node` so `tsc` can cover the runtime test suites | Project owner |
| A coverage tool for measured line/branch coverage | Project owner |
| JS end-to-end status fixture (trust anchor or public-CA sandbox) | P14-004 |
| Performance budgets and benchmarks | P13 (in progress) |
| Platform matrix, packaging, provenance | P14/P16 |

## 5. Conclusion

The repository is internally consistent after the seven fixes: the live documentation describes the engine that actually ships, the phase states match the registry and the indexes, and every quantitative claim traces to a run. The remaining work is the planned P13–P18 sequence plus the four owner decisions above, none of which blocks the current phase.
