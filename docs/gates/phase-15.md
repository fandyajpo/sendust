# QG-15 — Documentation Phase Record

**Decision:** APPROVED — project owner review, 2026-11-18
**Phase:** P15 — Documentation
**Gate:** PHASE-15
**Prepared:** 2026-11-18
**Branch:** `main`
**Evidence:** [quick start](../guides/quick-start.md),
[lifecycle and limits](../guides/lifecycle-limits.md),
[addresses, MIME, and attachments](../guides/addressing-mime-attachments.md),
[errors, security, and operations](../guides/errors-security-operations.md),
[maintainer guide](../guides/maintainer.md), the executable sample
`tests/docs/quickstart.mjs`, and `scripts/check-doc-claims.mjs`.

This record is the review package for PHASE-15. The documentation was written by
an agent, so the approval in section 6 is the required human review step, not a
self-approval. Nothing here approves P16 or any later gate.

## 1. Task evidence

| Task | Status | Evidence |
| --- | --- | --- |
| P15-001 | COMPLETED | Quick start: install, first send with `finally { close() }`, accepted/rejected/unknown and BUSY semantics, `unknown` is not a retry signal, and a loopback test relay. `tests/docs/quickstart.mjs` executes the sample (accepted send, close, `TRANSPORT_CLOSED` after close); it runs in `npm run verify` and the Native binding CI job. |
| P15-002 | COMPLETED | Lifecycle and limits: `open → closing → closed`, idempotent bounded close, BUSY with one connection, the 10/30/120/125 s budgets with lowering and ordering rules, and 100/10/20/30 MiB caps with inclusive boundary behaviour. |
| P15-003 | COMPLETED | Addresses, MIME, attachments: ASCII local parts, IDNA domains, Unicode display names, reserved headers, body shapes, path opt-in, regular-files-only, caps, filename handling, content-type map, and the error code for each rejected category. |
| P15-004 | COMPLETED | Errors, security, operations: two channels, every class and stable code with its caller action, redaction rules, TLS posture, outage/ambiguity/shutdown guidance, and a deployment checklist. |
| P15-005 | IN REVIEW | This record, plus the maintainer guide and the accuracy checker in section 3. |

## 2. Acceptance criteria

| Phase-15 criterion | Result | Evidence |
| --- | --- | --- |
| Quick start and API semantics are accurate and runnable | PASS | The guide's sample is executed by `tests/docs/quickstart.mjs` in CI; every lifecycle example closes |
| Lifecycle, timeout, cap, and BUSY behaviour match implementation exactly | PASS | `check-doc-claims.mjs` derives the caps from `ts/src/limits.ts` and the timeouts from `config/mod.rs` and fails on drift |
| Address, MIME, and attachment rules are complete and safe | PASS | Guide tables match the public contract; each rejection names its code; no URL form and no non-ASCII local part appears as valid |
| Error catalog and security guidance are accurate, including unknown handling | PASS | Every code named in a guide exists in the error model; `unknown` guidance forbids automatic resend without caller idempotency |
| Maintainer guide and links are valid; no unsupported claims exist | PASS | `check-docs.mjs` (links, indexes, registry parity) and `check-doc-claims.mjs` (claims, codes, numbers, closes) both pass |

## 3. Accuracy instruments

- **`tests/docs/quickstart.mjs`** — runs the quick start's example against a
  loopback TLS relay and the committed fixture CA: accepted send, explicit
  close, and `TRANSPORT_CLOSED` afterwards. Runs locally and in the Native
  binding CI job.
- **`scripts/check-doc-claims.mjs`** — fails when a guide:
  - names an error code that is not in the error model;
  - drifts from the caps or timeout defaults in the code;
  - claims delivery, automatic retries, pooling, SMTPUTF8, URL attachments, or a
    way to disable verification (negated statements are recognised);
  - shows a send without an explicit close.
- **`scripts/check-docs.mjs`** — links, task-registry parity, ADR and phase
  indexes, governance files, and pinned toolchain claims.
- **CI** — the checks job runs the structure check, the claims check, and the
  secrets scan; the Native binding job runs the doc sample.

## 4. Coverage of the phase plan

| Required by the plan | Where |
| --- | --- |
| Installation assumptions, transport configuration, one send, explicit close | Quick start sections 2–5 |
| Relay-accepted semantics, recipient statuses, BUSY, `unknown` | Quick start section 4 |
| Fake-relay and production-relay examples without real credentials | Quick start sections 3 and 5; executed sample |
| Lifecycle, idempotent close, 125 s bound, no GC shutdown | Lifecycle guide sections 1–2 |
| One active send, immediate BUSY, no queue or pool | Lifecycle guide section 3 |
| Timeout table and hard caps with boundary behaviour | Lifecycle guide sections 4–5 |
| ASCII local parts, IDNA, Unicode display names, SMTPUTF8 post-MVP | Addressing guide section 1 |
| Text/HTML alternatives, header encoding, reserved headers | Addressing guide sections 2–3 |
| Attachment opt-in, regular files only, caps, no URL form | Addressing guide section 4 |
| Error catalog with cause, action, and redaction | Errors guide sections 2–3 |
| TLS mandatory, credentials, no delivery guarantee, `unknown` handling | Errors guide sections 4–5 |
| Maintainer guide: layout, no-Node-core rule, commands, fixtures, release inputs | Maintainer guide |

## 5. Findings

| Severity | Finding | State |
| --- | --- | --- |
| Low | The first claims-checker run treated identifiers such as `PLATFORM_PACKAGES` and `SENDUST_HOST` as error codes, and asserted an unsound MiB equality | FIXED: the pattern is scoped to the documented code prefixes and the bad assertion is gone |
| Low | The errors guide's first send snippet lacked an explicit close, which the checker correctly rejected | FIXED: the snippet now closes in `finally` |
| Low | The maintainer guide's own rule used the phrase "delivery guarantee", which the claim scan flags | FIXED: reworded to "delivery claims" |

No high-severity finding. No claim of delivery, retry, pooling, SMTPUTF8, URL
attachment, or disabled verification survives the checker.

## 6. Decision

**APPROVED, 2026-11-18.** The project owner reviewed the guides and this record
and approved the PHASE-15 gate ("I approve PHASE-15"). The documentation was
written by an agent, so this approval is the required human review step, not a
self-approval. The reviewer accepted:

1. **The semantics.** `accepted` is relay acceptance, `unknown` is never an
   automatic retry signal, and BUSY is not a queue signal.
2. **The security posture.** Verification cannot be disabled, trust can only be
   extended additively through `tlsRootCertificate`, and no plaintext mode
   exists.
3. **The support statement.** Five targets, Node.js/Bun/Deno, with no
   unsupported platform or feature advertised, and the accuracy checks in
   section 3 as the drift guard.

P15-001 to P15-005 are COMPLETED and PHASE-15 is APPROVED. P16 becomes eligible.

Approval covers P15 only. It does not approve P16 or any later gate; a broken
link, a failing claim check, or a doc sample that no longer runs reopens this
gate.

## 7. Deferred and non-goals

- Deferred: a published-site or docs portal, examples beyond the runnable ones,
  translation, and release notes (P16).
- Non-goals: marketing claims, delivery guarantees, per-recipient success
  guarantees, retry or pool guidance, and documentation of post-MVP features as
  if they existed.
