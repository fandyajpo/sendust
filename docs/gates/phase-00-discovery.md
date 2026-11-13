# QG-00 — Discovery Integrity Gate

**Decision:** APPROVED  
**Phase:** P00 — Discovery  
**Reviewed:** 2026-11-13  
**Reviewer:** Independent architecture audit (GPT-6)

| Check | Result | Evidence |
| --- | --- | --- |
| Problem statement | PASS | [Discovery brief](../discovery/brief.md) |
| Constraint ledger | PASS | [Constraint ledger](../discovery/constraint-ledger.md), C-01 through C-30 mapped |
| Provenance | PASS | [Dependency research](../engineering/dependency-research.md), [lettre source validation](../research/lettre-0.11.23-validation.md) |
| Threat boundary | PASS | [Threat model](../discovery/threat-model.md), 14 abuse cases |
| Handoff | PASS | [P01 research gate](phase-01-research.md) |

## Decisions

- Product remains a standalone native email delivery engine for Node.js.
- SMTP relay acceptance is not mailbox delivery.
- Queue/job orchestration, workers, scheduling, persistence, and queue adapters remain HELD outside the current roadmap.
- No internal queue, automatic retry, connection pool, or per-recipient message fan-out.
- lettre 0.11.23 lower-level async connection API is selected for RCPT visibility.
- Runtime, Cargo compilation, fake SMTP/TLS, and Node integration checks remain downstream implementation evidence; this gate does not claim they have run.

**Decision: APPROVED.** Discovery artifacts are complete and research has unambiguous inputs. This is a documentation-readiness approval, not a claim of implemented behavior.