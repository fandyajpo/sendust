# Phase 07 — MIME

**Phase status:** IN PROGRESS  
**Phase dependency:** Phase 04  
**Companion references:** [system design](../architecture/system-design.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md)

## Objective

Build correct MVP MIME messages in the Rust core: headers, text bodies, HTML alternatives, Unicode encoding, filenames, and deterministic serialization that matches wire-size accounting.

## Scope

In scope: message construction, header encoding and injection defense, text/html alternative bodies, content disposition and filename encoding, deterministic boundaries, and MIME parse/render tests. Out of scope: attachment file loading (Phase 08), public API (Phase 09), SMTP transport changes, SMTPUTF8, and URL content.

## Tasks

### P07-001 — Implement header model and safe encoding

- **Size:** L
- **Status:** IN REVIEW
- **Prerequisites:** P04-002, P04-003
- **Description:** Build a header model for required and optional headers, with CRLF/LF/NUL rejection and RFC-compliant encoded words for Unicode display names and header values. Define header ordering and duplicate policy. Prevent message content from injecting headers or recipients.
- **Evidence criteria:** Tests cover ASCII and Unicode encoded words, long-line folding, duplicate header policy, and injection attempts using CRLF, bare LF, NUL, and folded whitespace. Exact rendered header bytes are asserted.

### P07-002 — Implement plain text and HTML alternative bodies

- **Size:** M
- **Status:** IN REVIEW
- **Prerequisites:** P07-001
- **Description:** Support text-only and multipart/alternative text+HTML bodies with correct charset declarations, transfer encoding, and deterministic boundaries. Define UTF-8 body handling and line-ending normalization policy. Reject unsupported content combinations explicitly.
- **Evidence criteria:** Golden MIME fixtures verify text-only and alternative messages byte-for-byte. UTF-8 bodies decode correctly. Boundary uniqueness is deterministic. Unsupported combinations produce stable validation errors.

### P07-003 — Implement filename and content-type handling policy

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P07-001
- **Description:** Define MIME content types, attachment filenames, Unicode filename encoding, path stripping, and dangerous-name handling without reading files. Phase 08 supplies bytes; Phase 07 owns the metadata model and rendering. Reject control characters and ambiguous filenames.
- **Evidence criteria:** Tests cover ASCII and Unicode filenames, long filenames, path separators, control characters, and duplicate filenames. Rendered parameters are RFC-compliant and contain no local path leakage.

### P07-004 — Implement deterministic serialization and wire-size parity

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P07-001, P07-002, P07-003
- **Description:** Serialize the final message exactly once and use those bytes for transport and the 30 MiB wire cap. Ensure deterministic boundaries and ordering across repeated runs. Compare MIME serializer output with the core size accounting contract.
- **Evidence criteria:** Repeated serialization yields identical bytes. Wire-size parity test asserts computed size equals serialized length for ASCII, UTF-8, encoded headers, and multipart bodies. Boundary fixtures at 30 MiB and 30 MiB + 1 byte behave per P04-003.

### P07-005 — MIME conformance and phase gate

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P07-001, P07-002, P07-003, P07-004
- **Description:** Validate the MIME output with local parsers and golden fixtures. Confirm no SMTPUTF8, no URL content, no header injection, and deterministic size behavior. Record any deferred MIME features as post-MVP.
- **Evidence criteria:** Golden fixtures pass for text-only, alternative, Unicode, long headers, and representative attachment metadata. Header-injection and control-character fixtures all fail safely. Phase 08 can consume attachment metadata without redefining MIME rules.

## Technical implementation details

MIME construction lives in `sendust-core` and is independent of Node. Display names and bodies may be Unicode; envelope local parts remain ASCII and domains IDNA. All headers reject CRLF/LF/NUL and are encoded safely. Bodies use UTF-8 with deterministic transfer encoding and stable boundaries. Attachment metadata includes filename and content type but not file access; files are loaded only in Phase 08 under explicit opt-in. Final serialization is the source of truth for the 30 MiB wire cap. No SMTPUTF8, URL fetching, pooling, retries, or delivery claims are introduced.

## Dependencies

Phase 07 requires Phase 04. P07-001 depends on P04-002 and P04-003; P07-002/003 depend on P07-001; P07-004 depends on the first three; P07-005 depends on all. Phase 08 consumes P07-005 and Phase 05; Phase 09 consumes MIME through Phase 08.

## Risks and edge cases

Encoded-word length limits, folding, duplicate headers, Unicode normalization, boundary collisions, filename ambiguity, and size-parity drift are key risks. A serializer that runs twice could produce different bytes if boundaries are random; determinism must be explicit.

## Testing requirements

Use golden fixtures for text-only, alternative, UTF-8, long folded headers, Unicode filenames, and multiple attachments. Injection fixtures include CRLF, bare LF, NUL, header-like display names, and path-like filenames. Assert exact bytes, parse-back equivalence, deterministic boundaries, and 30 MiB boundary behavior. No network or file reads occur in these tests.

## Acceptance criteria

- MIME messages render correctly and safely for all MVP body and metadata cases.
- Header and filename injection are prevented.
- Serialization is deterministic and equals wire-size accounting.
- No attachment bytes are read and no SMTP is invoked in this phase.

## Deliverables

Header model, body renderer, filename/content-type policy, deterministic serializer, golden MIME fixture suite, and MIME gate report.

## Completion gate

`PHASE-07` is approved only when P07-005 proves conformance, injection safety, and wire-size parity, and Phase 08 can attach bytes without changing MIME semantics.

## Evidence log

Evidence labels follow the [AI implementation contract](../engineering/ai-implementation-contract.md).

| Task | Evidence | State |
| --- | --- | --- |
| P07-002 | `mime/mod.rs`: `normalize_body` converts CRLF, bare CR and bare LF to CRLF before transfer encoding; text-only, HTML-only and `multipart/alternative` render with `charset=utf-8` and base64. `tests/mime_bodies.rs` pins byte-exact golden fixtures for text-only and alternative, and covers HTML-only, empty-string bodies, line-ending equivalence and the no-body rejection. `message/mod.rs` rejects a message with neither body as `VALIDATION_INVALID_OPTION`, matching the public API contract. | IMPLEMENTED, EXECUTED PASS locally; CI evidence pending |
| P07-001 | `message/mod.rs`: a custom header may appear once (case-insensitive), `Message-ID` and `Received` join the reserved set, and the model documents the fixed header order. `mime/headers.rs`: RFC 2047 encoded words that never split a character and never exceed 75 characters, ASCII words left unencoded, folding that only replaces an existing space so unfolding restores the value, unbreakable tokens falling back to encoded words instead of being split, and Unicode display names rendered as unquoted phrases. 10 header unit tests, ordering and exact-encoded-byte serialization tests, and 3 duplicate-policy tests. | IMPLEMENTED, EXECUTED PASS locally and in CI (all 8 workflows); CI evidence on the branch. |

## Status

IN PROGRESS. P07-001 and P07-002 are IN REVIEW; P07-003 to P07-005 are NOT STARTED.
