# Phase 04 — Rust core

**Phase status:** NOT STARTED  
**Phase dependency:** Phase 03  
**Companion references:** [system design](../architecture/system-design.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md), [reliability strategy](../engineering/reliability-strategy.md)

## Objective

Implement the Node-independent Rust core: validation, message model, address/header encoding, size accounting, transport state machine, timeout budgets, and SMTP orchestration through `lettre 0.11.23`. The core must have meaningful Rust unit tests and no Node linkage, so protocol behavior can be verified without a JavaScript runtime.

## Scope

In scope: `crates/sendust-core/src` modules for validation, addresses, limits, MIME-neutral message construction, attachment reads, state machine, deadlines, error taxonomy, and SMTP send orchestration. Out of scope: napi binding, TypeScript wrapper, release packaging, and post-MVP features such as SMTPUTF8, pooling, retries, URL attachments, and AbortSignal. SMTP dialogue uses lettre; no custom SMTP implementation.

## Tasks

### P04-001 — Implement core error taxonomy and deterministic validation order

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P03-005, P02-002
- **Description:** Create core-owned error types for validation, limits, address encoding, attachments, busy, closed, connect, TLS, authentication, protocol, and timeout. Implement a stable machine-readable code mapping and deterministic validation ordering so the same invalid input always returns the same first error. Keep errors Node-agnostic; no napi types in core.
- **Evidence criteria:** Unit tests cover every error variant and precedence pair. Codes are stable strings with documented categories. No error carries secrets or full message content. Errors compile without Node dependencies.

### P04-002 — Implement address, header, and Unicode/IDNA policy

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P03-005, P02-004
- **Description:** Implement envelope validation: ASCII local parts only, IDNA processing for domains, Unicode allowed in display names and message bodies, and SMTPUTF8 explicitly rejected as post-MVP. Encode display names and headers safely to prevent CRLF/header injection. Define required versus optional address forms and reject malformed, empty, overlong, or ambiguous inputs. Ensure header values cannot introduce new headers or recipients.
- **Evidence criteria:** Unit tests cover valid ASCII local/IDNA domain, Unicode display/body, and rejection of non-ASCII local parts. Injection fixtures include CRLF, bare LF, NUL, encoded-newline, and header-like display names. IDNA fixtures include punycode, uppercase, trailing dot policy, and invalid labels. Error precedence matches P04-001.

### P04-003 — Implement the message model and hard size accounting

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P03-005, P02-004
- **Description:** Build an internal message model with headers, body, recipients, and attachment descriptors. Enforce 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw attachment bytes, and 30 MiB wire message. Compute wire size from the final serialized representation, not an estimate, and enforce limits before network connect. Record inclusive boundary behavior and deterministic limit-error precedence.
- **Evidence criteria:** Unit tests assert 100 recipients pass and 101 fail; 10 MiB attachment passes and 10 MiB + 1 fails; 20 MiB aggregate passes and 20 MiB + 1 fails; 30 MiB wire passes and 30 MiB + 1 fails. Tests assert no SMTP connection is attempted on limit failure. Wire-size computation is covered with UTF-8, encoded headers, and attachment encodings.

### P04-004 — Implement transport state machine, deadlines, and BUSY semantics

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P03-005, P02-003
- **Description:** Implement `open -> closing -> closed` with exactly one active send, immediate `BUSY` for overlap, idempotent close, and deadline-bounded drain. Implement finite tunable budgets: connect 10s, operation 30s, total 120s, close 125s, with one shared total deadline preventing overrun. Close must be explicit and never GC-dependent. Exact stage attribution is emitted only where P01-003 proved it; otherwise timeouts are reported at the honest observable level.
- **Evidence criteria:** Unit tests cover send while open, BUSY on second send with no dial, send during closing, send after closed, double close, close during connect, and drain beyond 125s. Fake-time tests assert the 10s/30s/120s/125s boundaries. No queue is introduced. State transitions are exhaustively tested.

### P04-005 — Implement lettre-backed SMTP orchestration and core phase test gate

- **Size:** XL
- **Status:** NOT STARTED
- **Prerequisites:** P04-001, P04-002, P04-003, P04-004, P01-001, P01-003
- **Description:** Wire the core to `lettre 0.11.23` using the research-selected API and rustls `ring`/`webpki-roots` features. Implement connect, TLS, auth, MAIL FROM, RCPT TO, DATA, and final-response orchestration with relay-accepted semantics. Do not retry, do not pool, and do not implement a custom SMTP client. Implement ADR-0005 recipient policy using lettre's public AsyncSmtpConnection: observe every RCPT response, then send one DATA transaction only to the accepted-recipient subset. A recipient is accepted only after final DATA 2xx. Never fan out independent messages. Add core integration tests against the local fake SMTP/TLS harness.
- **Evidence criteria:** Tests cover successful acceptance, all-recipient rejection with no DATA, mixed `250`/`550` with one DATA transaction to the accepted subset, explicit final DATA `451` as `rejected`, plus missing final reply after DATA terminator as `unknown`, both with zero retries, auth failure, TLS verification failure, connect refusal, and abrupt server close. Success asserts relay acceptance only. No credentials appear in logs. Rust tests run without Node. The core phase gate passes only when all required fault fixtures execute locally.

## Technical implementation details

Core lives only in `crates/sendust-core/src` and must not depend on napi, Node headers, JS types, or a Node runtime. It uses `lettre 0.11.23` for SMTP with Tokio 1 futures driven by the caller’s runtime; core itself does not create a competing runtime. TLS uses the P01-002-proven rustls `ring` and `webpki-roots` features with verification always on. The send path enforces validation and size caps before connect. Envelope local parts are ASCII, domains are IDNA-processed, display names and bodies may be Unicode, and SMTPUTF8 is rejected. Limits are hard: 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw attachments, 30 MiB wire. Attachments are descriptors and bounded regular-file reads only when filesystem access is explicitly enabled; no URLs. One active send; overlap returns `BUSY` with no queue. Lifecycle is explicit `open -> closing -> closed`; close drains to 125s and never relies on GC. Timeouts are connect 10s, operation 30s, total 120s, close 125s, finite and tunable, with a single total deadline controlling overrun. Success means relay acceptance; an explicit final negative DATA reply is `rejected`; only a genuinely missing/indeterminate final disposition after DATA is `unknown`. Neither is retried. Recipient handling follows P01-003 and P06-003; no per-recipient independent messages.

## Dependencies

Phase 04 requires Phase 03 approval. P04-001/002/003/004 depend on P03-005 and their architecture contracts. P04-005 depends on all core modules plus P01-001 and P01-003. Phase 05 depends on P04-005. P06 later depends on P04-005 and P01-003 for SMTP edge behavior; Phase 04 must expose seams rather than implement binding behavior.

## Risks and edge cases

- Accidental napi usage in core would break the isolation guarantee.
- lettre feature unification can enable a different TLS backend; manifests must be checked.
- Wire-size calculation may diverge from actual serialization; tests must compare final bytes.
- IDNA mapping can change domain length and label validity; punycode boundaries need coverage.
- Timeouts implemented as independent timers can exceed total; shared deadlines are mandatory.
- Close during an in-flight lettre future may drop it at an unsafe point; drop behavior must be tested for leaks and hangs.
- The pinned lower-level API exposes RCPT replies; the remaining risk is whether mixed-recipient behavior matches the source contract under real SMTP dialogue. If not, block and supersede ADR-0005.
- Attachment reads may hit special files or change between stat and read; reject non-regular and bound reads.

## Testing requirements

Core tests are Rust unit/integration tests with no Node runtime. Required concrete fixtures:

- SMTP success: all recipients `250`, final DATA `250`, result `accepted`.
- Pre-DATA all-recipient rejection: every RCPT `550`, result `rejected`, no DATA command.
- Mixed RCPT: first `250`, second `550`; outcome must match P01-003 ADR exactly, with no per-recipient delivery claim.
- Explicit final DATA `451`: result `rejected`, no retry. Missing final reply after the DATA terminator may have reached the relay: result `unknown`, no retry.
- Auth failure: `535`; TLS failures: self-signed, expired, wrong host, incomplete chain, malformed.
- Network faults: connection refused, reset during greeting, reset during DATA, abrupt close before final response.
- Timeout boundaries: connect 10s vs 10s + epsilon, operation 30s vs 30s + epsilon, total 120s, close 125s.
- Limits at exact boundaries for 100 recipients, 10 MiB, 20 MiB, 30 MiB.
- Address fixtures: IDNA, Unicode display/body, non-ASCII local part rejection, CRLF/NUL injection.
- State fixtures: BUSY, close while connecting, close while sending, double close, send after close.
- Buffer-copy seam is core-level (owned bytes) and later reasserted at the binding boundary.

## Acceptance criteria

- Core implements all MVP validation, limits, state, timeout, and SMTP orchestration with relay-accepted semantics.
- Core has no Node linkage and all tests run as Rust tests.
- Recipient behavior matches ADR-0005: explicit RCPT refusals are preserved, accepted recipients share one DATA transaction, and no per-recipient message fan-out occurs.
- No custom SMTP, pooling, retries, SMTPUTF8, URL attachments, or GC-dependent close.
- Fault fixtures pass locally without external network.

## Deliverables

- Core error, address, message, limits, attachment, state, deadline, and SMTP modules.
- Rust unit and integration test suites with fake SMTP/TLS fixtures.
- Core phase gate report with fault-matrix results.

## Completion gate

Gate `PHASE-04` is `APPROVED` only when P04-005 passes all Rust tests, the dependency-direction guard still proves no Node linkage, and the recipient/unknown/busy/timeout/limit semantics exactly match Phase 02. Phase 05 may begin only after this gate.

## Modular core implementation requirements

Implement the approved ADR-0016 module responsibilities without creating additional crates. Keep domain modules independent of lettre/SMTP/napi/runtime APIs. Put SMTP protocol behavior behind the internal delivery contract in the SMTP adapter. Add focused unit tests per module and dependency checks. The exact file layout may evolve, but ownership and dependency direction may not.

## Status

NOT STARTED. No core code has been written.
