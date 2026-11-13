# Phase 06 — SMTP

**Phase status:** NOT STARTED  
**Phase dependency:** Phase 04, Phase 01  
**Companion references:** [system design](../architecture/system-design.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md), [reliability strategy](../engineering/reliability-strategy.md)

## Objective

Implement SMTP behavior through `lettre 0.11.23` with honest relay-accepted semantics, fail-closed TLS, aggregate recipient outcomes, and no retry or per-recipient message splitting. This phase resolves the remaining recipient-observability question in code and tests.

## Scope

In scope: relay session configuration, TLS and authentication, MAIL/RCPT/DATA transaction handling, response normalization, unknown-after-DATA behavior, and the P06-003 recipient-policy validation. Out of scope: MIME construction, attachment file access, TypeScript API, pooling, automatic retries, SMTPUTF8, and custom SMTP implementation.

## Tasks

### P06-001 — Configure lettre 0.11.23 relay session and envelope submission

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P04-005, P01-001
- **Description:** Implement relay connection configuration from validated core options: host, port, EHLO identity, optional credentials, and required TLS mode. Submit one MAIL command and one RCPT command per recipient using lettre's selected lower-level async API, then one DATA transaction for the accepted-recipient subset. Do not implement SMTP commands manually. Define behavior for missing/invalid relay configuration before any network attempt.
- **Evidence criteria:** Tests prove valid configuration reaches the fake relay and invalid host/port/EHLO/auth options fail before send. No custom SMTP command path exists. The implementation cites the P01-001 API choice.

### P06-002 — Enforce TLS verification and authentication failure semantics

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P06-001, P01-002
- **Description:** Wire rustls with the P01-002-proven `ring` and `webpki-roots` feature set. Certificate and hostname verification are mandatory and not configurable off. Map TLS, certificate, and authentication failures to closed, non-retryable errors without leaking credentials. Support the relay authentication modes approved in architecture; reject unsupported modes explicitly.
- **Evidence criteria:** Self-signed, expired, wrong-host, incomplete-chain, and malformed-certificate fixtures all fail closed before DATA. No public or internal option disables verification. Authentication failure returns a stable error and never logs the password.

### P06-003 — Validate and implement recipient partial-failure policy

- **Size:** XL
- **Status:** NOT STARTED
- **Prerequisites:** P01-003, P06-001
- **Description:** Implement the P01-003 / ADR-0005 selected behavior: use AsyncSmtpConnection.command(Rcpt::new(...)) and Error::status() to observe each recipient reply. Continue after ordinary explicit 4xx/5xx RCPT refusal; if 421 is returned, stop immediately and do not issue DATA. If no recipient is accepted, do not issue DATA. Otherwise send exactly one DATA transaction to the accepted subset. No fallback to whole-envelope rejection and no per-recipient message fan-out.
- **Evidence criteria:** Mixed fixture first `250`, second `550` records the first recipient as accepted only after final DATA 2xx and the second as rejected. Exactly one DATA transaction is sent to the accepted subset. When every RCPT reply is negative, no DATA command is sent. A 421 RCPT response terminates the session and prevents DATA even if earlier RCPT responses were 2xx. No per-recipient independent message is ever sent.

### P06-004 — Normalize SMTP responses, unknown-after-DATA, and no-retry behavior

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P06-001, P06-002
- **Description:** Map SMTP reply classes to core results: relay acceptance for a successful final DATA response, definite rejection before acceptance where certainty exists, and `unknown` after DATA when the final outcome cannot be known. Implement zero automatic retries, including after timeouts, resets, or ambiguous replies. Preserve attempt counts without creating duplicate sends. Define how protocol violations and abrupt closes map without pretending certainty.
- **Evidence criteria:** An explicit final DATA `451` yields `rejected`; a dropped connection after the terminator may have reached the relay and before a final reply yields `unknown`. Both use exactly one SMTP transaction. `550` before DATA yields `rejected`; `250` final response yields `accepted` meaning relay accepted, not delivered. A reset during DATA yields `unknown`. No retry loop exists in code or test.

### P06-005 — SMTP fault-matrix phase gate

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P06-001, P06-002, P06-003, P06-004
- **Description:** Run the complete SMTP fault matrix against the deterministic loopback harness and review the recipient ADR implementation. Confirm every result is honest, TLS cannot be weakened, no retry/pooling exists, and timeout stages are reported only where observable.
- **Evidence criteria:** Gate covers success, all RCPT rejected, mixed RCPT, explicit final `451` and missing-final-reply-after-DATA, auth `535`, five TLS certificate failures, connection refused, greeting reset, DATA reset, abrupt pre-final close, connect 11s, command 31s, total 121s, and close 126s. Phase 07 and Phase 09 may consume SMTP only after approval.

## Technical implementation details

SMTP is exclusively `lettre 0.11.23`; no custom protocol client is permitted unless Phase 01 explicitly justified it, and the default decision is no custom SMTP. Relay configuration is core-owned and validated before connect. TLS uses rustls with `ring` and `webpki-roots`; verification is always enabled. Authentication failures are terminal. A transport handles one active send; `BUSY` is enforced by the lifecycle layer. Success means the relay accepted the message; it never means mailbox delivery. After DATA, ambiguity maps to `unknown` with no retry. Recipient handling follows ADR-0005: observe each RCPT response, send one DATA transaction to the accepted subset, and report recipient accepted only after final DATA 2xx. If the pinned API behavior differs from source review, block and supersede ADR-0005; do not silently change the contract. Timeout defaults remain connect 10s, operation 30s, total 120s, close 125s. There is no pooling, no retry, and no SMTPUTF8.

## Dependencies

Phase 06 requires Phase 04 and Phase 01 gates. P06-001 needs P04-005 and P01-001; P06-002 needs P01-002; P06-003 needs P01-003 and P06-001; P06-004 needs P06-001/002; P06-005 depends on all SMTP tasks. Phase 09 consumes P06-005; Phase 08 may proceed independently after its own inputs.

## Risks and edge cases

- The high-level lettre transport may not expose RCPT outcomes; promising partial success is forbidden.
- A server may accept RCPT and then explicitly reject the final DATA with 4xx/5xx; result is `rejected`. If the final response is missing after the terminator may have reached the relay, result is `unknown`.
- Greeting, EHLO, STARTTLS, AUTH, MAIL, RCPT, DATA, and final response may fail at different points; certainty depends on observable stage.
- Retrying an unknown result can duplicate mail and is prohibited.
- TLS feature drift or permissive roots would violate security posture.
- Abrupt server closure can look like a timeout; both remain honest failures.
- Credentials may appear in debug errors; redaction is mandatory.

## Testing requirements

Use loopback fake SMTP/TLS only, with a scripted dialogue and no arbitrary sleeps. Required fixtures: all `250`; all RCPT `550`; mixed `250`/`550`; RCPT `421` after one accepted RCPT; `451` after final DATA; `535` auth; self-signed, expired, wrong-host, incomplete-chain, and malformed TLS; connection refused; reset during greeting; reset during DATA; abrupt close before final response; connect stall 11s; command stall 31s; total stall 121s; close drain 126s. Assert exact command trace for each, including no DATA when all RCPT commands are rejected, one DATA for mixed RCPT, and no second transaction after unknown. Assert credentials never appear in error text or logs.

## Acceptance criteria

- SMTP uses lettre only and follows the research-selected API.
- TLS verification is mandatory and all certificate failures close the connection.
- Recipient policy matches the explicit ADR and never promises high-level partial status.
- Accepted, rejected, and unknown results are honest; no retry exists.
- Fault matrix passes with exact command traces.

## Deliverables

Relay configuration, TLS/auth enforcement, recipient-policy implementation, SMTP response mapping, and SMTP fault-matrix gate report.

## Completion gate

Gate `PHASE-06` is `APPROVED` only when P06-005 proves the full fault matrix, the recipient ADR is explicit and implemented, no retry/pooling/custom SMTP exists, and the result contract remains relay-accepted.

## SMTP adapter boundary requirements

All SMTP protocol interaction, TLS, AUTH, RCPT/DATA and SMTP response mapping must remain inside the SMTP adapter. Delivery orchestration must call the narrow internal delivery port and must not depend on lettre types. lettre is the sole MVP SMTP implementation. No public transport plugin registry or alternate provider is introduced.

## Status

NOT STARTED. No SMTP code has been written.
