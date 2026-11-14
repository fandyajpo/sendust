# ADR-0005 — Recipient partial-failure policy

**Status:** ACCEPTED (architecture baseline; amended by ADR-0017, policy unchanged)  
**Date:** 2026-11-13  
**Deciders:** Research gate (P01-005), architecture review (P02-005)  
**Validated by:** P01-003 source inspection; executable validation P06-003, P06-004 and P09-002

## Context

SMTP can accept some RCPT TO commands and reject others within one transaction. Product expectations often assume per-recipient results, but the high-level lettre transport may not expose per-recipient RCPT status. Promising partial success without evidence would be a false contract; sending one message per recipient to manufacture it would violate the single-envelope design and multiply SMTP transactions.

## Decision

**Amended by ADR-0017:** the policy below is unchanged. The mechanism that makes it observable is now the native SMTP client, whose parser returns the exact reply for every RCPT command; lettre's lower-level API was the original evidence and served as the differential oracle during P06, then was removed.

Use lettre 0.11.23's public lower-level `AsyncSmtpConnection` API. `command(Rcpt::new(...))` returns an individual response or an SMTP error with a public numeric `status()`. Source inspection confirms this API is sufficient to observe each RCPT response. Do not use the high-level transport for Sendust's send path.

### Selected policy — per-recipient status is observable

Issue one RCPT command for each recipient and record the exact response. Continue collecting ordinary 4xx/5xx recipient refusals; do not send DATA to recipients refused by the relay. Treat 421 as a session-closing response: stop immediately, issue no DATA, and reject the transaction. If at least one recipient is accepted at RCPT and no 421 occurred, send one DATA transaction to the accepted-recipient envelope. A recipient is reported `accepted` only after a positive final DATA response. Explicit RCPT refusals remain `rejected`. If the final DATA reply is explicitly negative, the overall result is `rejected` and all RCPT-accepted recipients are also `rejected`. If the final disposition is unknown, RCPT-accepted recipients become `unknown`; explicit RCPT refusals remain `rejected`. The overall result is `accepted` only after positive final DATA for at least one accepted recipient.

### Rejected fallback — if a future engine change removes recipient visibility

Do not silently fall back to guessed results. Block the upgrade and require a superseding ADR before implementation. The current pinned API exposes RCPT responses, so this fallback is not selected.

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

- The public result type distinguishes `accepted`, `rejected`, and `unknown` for recipients. `accepted` means RCPT accepted and final DATA positive; RCPT acceptance alone is not message acceptance.
- Tests must assert exactly one DATA transaction for the RCPT-accepted subset, and no DATA when all RCPT commands are rejected.
- Documentation must state the selected behavior and never imply per-recipient delivery.
- A future engine change that hides RCPT replies is blocked until an ADR explicitly revises this contract.

## Evidence required

- Stage/recipient observability table citing the native client's RCPT path.
- Mixed fixture (`250` then `550`) command trace showing one DATA transaction addressed only to the accepted recipient.
- Explicit final `451` after DATA fixture showing a definitive `rejected` outcome.
- Separate dropped-connection/no-final-reply fixture after DATA terminator showing `unknown` and no retry.
- A gate record naming the inspected API or documenting its absence.
