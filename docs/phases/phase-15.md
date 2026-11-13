# Phase 15 — Documentation

**Phase status:** NOT STARTED  
**Phase dependency:** Phase 09  
**Companion references:** [system design](../architecture/system-design.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md), [reliability strategy](../engineering/reliability-strategy.md), [release strategy](../engineering/release-strategy.md)

## Objective

Produce accurate user and maintainer documentation for the MVP public API, semantics, limits, security posture, lifecycle, attachments, errors, and support matrix. Documentation must not promise delivery, per-recipient success, retries, pooling, or unsupported platforms.

## Scope

In scope: API reference for `ts/src`, quick start, relay-accepted semantics, lifecycle/close guidance, timeout and cap documentation, attachment opt-in, error catalog, security notes, limitations, and maintenance/testing guide. Out of scope: changing code, marketing claims, publication, and post-MVP feature documentation beyond explicit “not included” notes.

## Tasks

### P15-001 — Write the quick start and semantic contract

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P09-005
- **Description:** Document installation assumptions, transport configuration, one successful send, and explicit close. Define “success” as relay acceptance, not delivery. Explain overall `accepted`, `rejected`, `unknown`, recipient `accepted`/`rejected`/`unknown`, and `BUSY`, and state that explicit final negative DATA replies are `rejected`, missing/ambiguous final replies after DATA may have reached the relay are `unknown`, and neither is retried. Include a minimal fake-relay example and a production-relay example without real credentials.
- **Evidence criteria:** Every code sample compiles against the public types and runs locally with the fake relay. Semantics section contains no delivery guarantee. BUSY and unknown are explained with transaction-level behavior. Close appears in every lifecycle example.

### P15-002 — Document lifecycle, timeouts, concurrency, and limits

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P15-001
- **Description:** Explain `open -> closing -> closed`, idempotent close, the 125s close/drain bound, no GC-dependent shutdown, one active send, immediate BUSY with no queue, and finite tunable timeouts (connect 10s, operation 30s, total 120s, close 125s). Document hard caps: 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw, 30 MiB wire. Include operational guidance for callers and explicitly state no pooling and no automatic retries.
- **Evidence criteria:** Docs match P02/P09 contracts exactly. Timeout table and cap table are complete with boundary behavior. BUSY guidance says do not queue internally. No wording implies retry or delivery confirmation.

### P15-003 — Document addresses, MIME, and attachments

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P15-001
- **Description:** Explain ASCII local parts, IDNA domains, Unicode display names and bodies, and SMTPUTF8 as post-MVP. Document text/HTML alternatives, header encoding, filename rules, and attachment opt-in. State that filesystem access is disabled by default, only bounded regular files are allowed, and URL/remote attachment fetching is unsupported. Include examples of accepted and rejected inputs.
- **Evidence criteria:** Address and attachment examples align with validation tests. Every rejected category is documented with its error code. No example uses a URL attachment or non-ASCII local part. The opt-in requirement is prominent.

### P15-004 — Write the error catalog and security/operations guide

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P15-001, P11-005
- **Description:** Document every public error class/code, cause, retryability posture, and redaction behavior. Explain TLS verification is mandatory, credentials must be protected, logs are redacted, and the library does not guarantee delivery. Add operational notes for relay outages, ambiguous outcomes, close during shutdown, and safe handling of `unknown`.
- **Evidence criteria:** Error catalog matches P09/P11 exactly. Every code has a description and caller action. Security guide states verification cannot be disabled and no plaintext fallback exists. `unknown` guidance explicitly forbids automatic resend without caller-level idempotency reasoning.

### P15-005 — Documentation accuracy gate and maintainer guide

- **Size:** S
- **Status:** NOT STARTED
- **Prerequisites:** P15-001, P15-002, P15-003, P15-004
- **Description:** Validate docs against types, tests, and the support matrix. Add a maintainer guide covering the two-crate layout, no-Node-core rule, test commands, fake relay fixtures, and release inputs. Link the architecture and engineering strategies. Record known limitations and unsupported platforms.
- **Evidence criteria:** Doc samples compile or are validated against tests. Link check passes for relative strategy docs. Maintainer guide accurately describes `crates/sendust-core/src`, `crates/sendust-napi/src`, `ts/src`, and `tests/`. No unsupported feature or platform is advertised. Phase 16 may consume P15-005.

## Technical implementation details

Documentation describes the approved architecture baseline; it must not claim implementation behavior until the relevant implementation gates pass: `TS wrapper -> napi -> core -> lettre`, `lettre 0.11.23`, rustls `ring`/`webpki-roots`, Tokio 1 managed by napi, and two crates with core free of Node linkage. Success means relay acceptance, never delivery. One active send returns `BUSY`; there is no queue, pooling, or automatic retry. Close is explicit, idempotent, and bounded by 125s, not GC-dependent. Timeouts are connect 10s, operation 30s, total 120s, close 125s, finite and tunable. Caps are 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw, 30 MiB wire. Envelope local parts are ASCII, domains IDNA, display/body Unicode; SMTPUTF8 is post-MVP. Attachments require opt-in, allow bounded regular files only, and never URLs. JS buffers are copied before async. AbortSignal is optional and not required.

## Dependencies

Phase 15 requires Phase 09. P15-001 depends on P09-005; P15-002 and P15-003 depend on P15-001; P15-004 adds P11-005; P15-005 depends on all. Phase 16 requires P15-005 along with Phase 14, Phase 12, and Phase 13.

## Risks and edge cases

Documentation can drift from types or imply delivery. Examples may accidentally use non-ASCII local parts or URL attachments. Error guidance may encourage retrying `unknown`. Support statements may overpromise Node versions. Operational advice may suggest GC-based shutdown. Relative links may break before other docs exist.

## Testing requirements

Validate every code sample against the public types and local fake relay. Add doc-link checks for `../architecture/system-design.md` and `../engineering/*-strategy.md`. Run example tests for accepted, rejected, mixed RCPT, unknown, BUSY, close, attachment opt-in, and error redaction. Search docs for forbidden claims: delivery guarantee, automatic retry, pooling, SMTPUTF8 support, URL attachments, verification disable. Confirm no real credentials appear.

## Acceptance criteria

- Quick start and API semantics are accurate and runnable.
- Lifecycle, timeout, cap, and BUSY behavior match implementation exactly.
- Address, MIME, and attachment rules are complete and safe.
- Error catalog and security guidance are accurate, including unknown handling.
- Maintainer guide and links are valid; no unsupported claims exist.

## Deliverables

Quick start, semantic contract, lifecycle/timeout/limits guide, address/MIME/attachment guide, error catalog, security/operations guide, maintainer guide, and documentation accuracy gate report.

## Completion gate

`PHASE-15` is approved only when P15-005 proves documentation accuracy against types and tests, valid links, and no delivery/retry/pooling/unsupported-feature claims. Phase 16 may begin after approval.

## Status

NOT STARTED.
