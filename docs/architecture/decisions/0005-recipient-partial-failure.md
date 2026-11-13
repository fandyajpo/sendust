# ADR-0005 — Recipient partial-failure policy

**Status:** BLOCKED on P01-003 and P06-003  
**Date:** 2026-11-13  
**Deciders:** Research gate (P01-005), architecture review (P02-005)  
**Validated by:** P01-003, P06-003, P09-002

## Context

SMTP can accept some RCPT TO commands and reject others within one transaction. Product expectations often assume per-recipient results, but the high-level lettre transport may not expose per-recipient RCPT status. Promising partial success without evidence would be a false contract; sending one message per recipient to manufacture it would violate the single-envelope design and multiply SMTP transactions.

## Decision (provisional)

P01-003 must inspect the lower-level lettre async connection API. Exactly one of the following applies:

### Option A — per-recipient status is observable

If the selected API exposes each RCPT response and the DATA transition, report recipient statuses honestly. A recipient accepted at RCPT but part of an envelope whose DATA failed is `unknown`; a recipient rejected at RCPT is `rejected`. The overall result remains `accepted` only after positive final DATA.

### Option B — per-recipient status is not observable (default if unproven)

Reject the entire envelope before DATA if any recipient is rejected. Report the overall result as `rejected`, with recipient statuses limited to what is actually known (`rejected` for explicitly rejected addresses if the API exposes them, otherwise `attempted`). Never claim per-recipient success.

### Forbidden in all cases

- Sending one independent message per recipient.
- Reporting `accepted` for a recipient whose envelope failed.
- Retrying a partial or unknown transaction.
- Inferring delivery from RCPT acceptance.

## Alternatives

1. **Fan out one message per recipient.** Simple mental model, but changes semantics, multiplies load, breaks BCC privacy assumptions, and violates the single-transaction design.
2. **Always reject the whole envelope.** Honest and simple, but may discard accepted recipients that the relay would have delivered.
3. **Promise partial success from the high-level API without proof.** Rejected: unsupported claim.

## Consequences

- The public result type must distinguish `accepted`, `rejected`, `attempted`, and `unknown` even if some combinations are unreachable under Option B.
- Tests must assert exactly one SMTP transaction and no DATA after a pre-DATA rejection.
- Documentation must state the selected behavior and never imply per-recipient delivery.
- If Option B is selected, it is a documented MVP limitation and a post-MVP candidate for improvement.

## Evidence required

- Stage/recipient observability table citing lettre source or API documentation.
- Mixed fixture (`250` then `550`) command trace.
- Post-DATA `451` fixture showing `unknown` and no retry.
- A gate record naming the inspected API or documenting its absence.
