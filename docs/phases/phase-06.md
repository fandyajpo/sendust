# Phase 06 — SMTP engine

**Phase status:** IN PROGRESS  
**Phase dependency:** Phase 04  
**Companion references:** [system design](../architecture/system-design.md), [ADR-0017](../architecture/decisions/0017-native-smtp-engine.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md), [reliability strategy](../engineering/reliability-strategy.md)

## Objective

Own the SMTP engine: a Sendust-implemented client for command serialization, response parsing, capability negotiation, STARTTLS, AUTH, and the MAIL/RCPT/DATA transaction, behind the existing `DeliveryPort`, with relay-accepted semantics, fail-closed TLS, aggregate recipient outcomes, honest unknown-after-DATA, and no retry or per-recipient fan-out. Trusted libraries remain for TLS (`rustls` + `ring` + `webpki-roots`), Tokio I/O and timers, `base64` and `idna`.

## Scope

In scope: the SMTP protocol codec, the multiline response parser, EHLO capabilities, implicit TLS and STARTTLS, AUTH PLAIN/LOGIN, the session state machine, MAIL/RCPT/DATA and QUIT, response normalization, unknown-after-DATA, and the fault matrix. Out of scope: MIME construction, attachment file access, TypeScript API, pooling, retries, queues, SMTPUTF8, DKIM, CHUNKING/BDAT, PIPELINING, DSN, and any third-party SMTP engine.

## Tasks

### P06-001 — Implement the SMTP command codec and response parser

- **Size:** L
- **Status:** IN REVIEW
- **Prerequisites:** P04-005
- **Description:** Implement Sendust-owned serialization for EHLO, STARTTLS, AUTH, MAIL, RCPT, DATA, RSET, NOOP and QUIT, and a bounded parser for single and multiline replies (code class, continuation, last line, enhanced status when present). Enforce RFC 5321 line and response caps, reject malformed replies, and represent results in Sendust types. Keep the module I/O-free so it is unit-testable without a socket. No lettre type may appear.
- **Evidence criteria:** Unit tests cover every command's exact bytes, single and multiline replies, continuation without a final line, oversized lines and responses, invalid codes, and enhanced-status extraction. No hand-written command string exists outside the codec. Architecture check still proves lettre is confined to no module.

### P06-002 — Implement the native connection, implicit TLS, and STARTTLS client

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P06-001
- **Description:** Connect with Tokio over one resolved relay address, read the greeting, send EHLO, and parse capabilities. Support implicit TLS and a required STARTTLS upgrade using rustls with `ring` and `webpki-roots`, SNI and hostname verification always on. Re-EHLO after STARTTLS. Never fall back to plaintext, never treat a missing STARTTLS advertisement as success, and never disable verification.
- **Evidence criteria:** Loopback harness proves greeting, EHLO, implicit TLS, STARTTLS upgrade, and re-EHLO. Self-signed, expired, wrong-host, incomplete-chain and malformed fixtures all fail closed with distinct codes; no configuration can disable verification.

### P06-003 — Implement SMTP AUTH and the session state machine

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P06-001, P06-002
- **Description:** Implement AUTH PLAIN and LOGIN after TLS, including the 334 challenge exchange with a bounded challenge count, and a session state machine connecting the parsed replies to MAIL, one RCPT per recipient, DATA, and QUIT. AUTH is refused before TLS; unsupported mechanisms fail with a stable code. Map pre-transaction failures to connection, TLS or authentication errors, never to a delivery outcome.
- **Evidence criteria:** Tests cover successful PLAIN and LOGIN, 535 rejection, no advertised mechanism, challenge exhaustion, AUTH attempted before TLS, and every state transition including early close and protocol violations. Credentials never appear in errors or logs.

### P06-004 — Swap the delivery adapter and prove the differential fault matrix

- **Size:** XL
- **Status:** NOT STARTED
- **Prerequisites:** P06-003, P04-005
- **Description:** Implement `DeliveryPort` over the native session behind the unchanged `SmtpConfig`, `SmtpTransport` and `SmtpSession` surface, preserving ADR-0005 recipient semantics, ADR-0006 lifecycle, ADR-0007 deadlines and the existing outcome/error mapping. While migrating, run the fault matrix against both the native client and the lettre backend as a dev-only oracle; then remove lettre from production dependencies.
- **Evidence criteria:** Identical outcomes and stable codes from both backends for the whole matrix; one DATA transaction for the accepted subset; no DATA when every RCPT is refused; 421 stops the envelope; explicit final `451` is `rejected`; missing final reply after the terminator is `unknown`; no retry path exists. `cargo tree` proves lettre is absent from production dependencies.

### P06-005 — Native SMTP engine phase gate

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P06-001, P06-002, P06-003, P06-004
- **Description:** Run the complete fault matrix against the native engine and review the parser's security posture: bounded input, smuggling resistance, STARTTLS downgrade resistance, truncation handling, redaction, and stage-accurate error mapping. Confirm no retry, no pooling, no third-party engine, and no weakened TLS.
- **Evidence criteria:** Gate covers success, all-RCPT rejection, mixed RCPT, explicit final `451`, missing final reply after DATA, AUTH `535`, five certificate failures, connection refused, greeting reset, DATA reset, abrupt pre-final close, connect 11s, command 31s, total 121s, and close 126s. Includes a parser security review and a final `cargo tree` showing only the approved dependency set. Phases 07 and 09 may consume SMTP only after approval.

## Technical implementation details

```text
transports/smtp/
  mod.rs            public config/types and the DeliveryPort impl (unchanged surface)
  client/           Tokio TCP connect and tokio-rustls upgrade (implicit TLS, STARTTLS)
  proto/            command codec, response parser, capabilities, AUTH, dot-stuffing
  session.rs        connect -> greeting -> EHLO -> [STARTTLS -> EHLO] -> [AUTH] -> MAIL -> RCPT* -> DATA -> QUIT
  transport.rs      delivery orchestration over the session and SendDeadlines
  error.rs          mapping into the existing ErrorCode/Stage taxonomy
```

Relay configuration stays core-owned and is validated before connect. TLS uses rustls with `ring` and `webpki-roots`; verification is always enabled. A transport handles one active send; `BUSY` is enforced by the lifecycle layer. Success means the relay accepted the message, never mailbox delivery. After DATA, ambiguity maps to `unknown` with no retry. Recipient handling follows ADR-0005: observe each RCPT response, send one DATA transaction to the accepted subset, and report a recipient accepted only after a final DATA 2xx. Timeout defaults remain connect 10s, operation 30s, total 120s, close 125s. There is no pooling, no retry, no SMTPUTF8, and no third-party SMTP implementation.

## Dependencies

Phase 06 requires the Phase 04 gate and accepted ADR-0017. P06-001 needs P04-005; P06-002 needs P06-001; P06-003 needs P06-001 and P06-002; P06-004 needs P06-003 and P04-005; P06-005 depends on all four. Phase 09 consumes P06-005; Phase 07 may proceed independently after P04.

## Risks and edge cases

- Parser correctness and adversarial input: oversized lines, truncated replies, smuggling, invalid codes.
- STARTTLS downgrade and re-EHLO regressions; verification must never be bypassed.
- Regression against a previously proven engine; the differential oracle is mandatory until parity.
- Provider quirks in greeting, capabilities and reply text.
- Enhanced-status parsing may change error detail shape; codes remain append-only.
- AUTH challenge loops and credential redaction.
- Ambiguity after the DATA terminator must remain `unknown` and never be retried.

## Testing requirements

Loopback fake SMTP/TLS only, scripted dialogue, no arbitrary sleeps. Required fixtures: all `250`; all RCPT `550`; mixed `250`/`550`; RCPT `421` after one accepted RCPT; `451` after final DATA; `535` auth; PLAIN and LOGIN success; no advertised mechanism; self-signed, expired, wrong-host, incomplete-chain and malformed TLS; STARTTLS advertised, refused, absent and stripped; connection refused; reset during greeting; reset during DATA; abrupt close before final response; oversized and malformed replies; connect stall 11s; command stall 31s; total stall 121s; close drain 126s. Assert exact command traces, no DATA when all RCPT commands are rejected, one DATA for mixed RCPT, and no second transaction after unknown. Assert credentials never appear in error text or logs.

## Acceptance criteria

- The SMTP engine is implemented in Sendust and no third-party SMTP engine is a production dependency.
- TLS verification is mandatory and every certificate failure closes before DATA.
- Recipient policy matches ADR-0005 and never fans out per recipient.
- Accepted, rejected and unknown results are honest; no retry exists.
- The differential fault matrix passes with exact command traces.

## Deliverables

Native SMTP codec and parser, TLS/STARTTLS client, AUTH and session state machine, native delivery adapter, differential fault-matrix evidence, and the phase gate report.

## Completion gate

Gate `PHASE-06` is `APPROVED` only when P06-005 proves the full fault matrix, the parser security review, no retry/pooling/third-party engine, and the unchanged relay-accepted result contract.

## SMTP adapter boundary requirements

All SMTP protocol interaction, TLS, AUTH, RCPT/DATA and response mapping remain inside the SMTP adapter. Delivery orchestration calls the narrow internal delivery port and must not depend on protocol types. Sendust's native client is the sole MVP SMTP implementation; no public transport plugin registry or alternate provider is introduced.

## Evidence log

Evidence labels follow the [AI implementation contract](../engineering/ai-implementation-contract.md).

| Task | Evidence | State |
| --- | --- | --- |
| P06-001 | `crates/sendust-core/src/transports/smtp/proto/` — `command.rs` (EHLO, STARTTLS, AUTH PLAIN/LOGIN, MAIL, RCPT, DATA, RSET, NOOP, QUIT with injection rejection and the 512-byte cap), `response.rs` (three-digit code validation, multiline builder, 1000-byte line and 64 KiB reply caps, RFC 3463 enhanced status), `capabilities.rs` (STARTTLS, AUTH mechanisms, SIZE, 8BITMIME). 15 new unit tests; 109 core tests total, `cargo clippy -D warnings`, architecture/docs/secrets checks all pass locally. The module is I/O-free and carries a temporary `dead_code` allow until P06-002/P06-003 wire it in. | IMPLEMENTED, EXECUTED PASS locally; CI evidence pending |

## Status

IN PROGRESS. P06-001 is IN REVIEW; P06-002 to P06-005 are NOT STARTED.
