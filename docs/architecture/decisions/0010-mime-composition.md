# ADR-0010 — MIME composition in core with lettre primitives

**Status:** ACCEPTED (architecture baseline); conformance validation required at P07-005  
**Date:** 2026-11-13  
**Deciders:** Architecture review (P02-005)  
**Validated by:** P07-001–P07-005

## Context

Correct MIME is a product requirement. lettre provides a typed message builder and header types; using it reduces encoding mistakes. However, Sendust needs deterministic structure, wire-size accounting, BCC suppression, custom header policy, and testable serialization. The boundary between “use lettre’s builder” and “compose in core” must be explicit.

## Decision

- The message model and validation live in `sendust-core`.
- MIME structure is composed in core. lettre message/header primitives may be used for encoding individual values where they are correct and testable.
- Core owns:
  - header ordering and folding policy,
  - BCC suppression,
  - `multipart/alternative` and `multipart/mixed` structure,
  - transfer-encoding selection,
  - filename encoding,
  - final wire-size accounting.
- The final serialized bytes are parsed by an independent parser (`mail-parser`, test-only) in conformance tests.
- No hand-rolled base64/quoted-printable implementation unless a proven defect requires it.

## Alternatives

1. **Fully delegate to lettre’s `Message` builder.** Less code, but less control over deterministic structure, custom headers, and wire accounting; also couples the contract to lettre internals.
2. **Fully custom MIME.** Maximum control, high encoding and folding risk.
3. **Hybrid (chosen).** Core owns policy and structure; reuse vetted primitives for encoding.

## Consequences

- Core tests can assert exact MIME bytes and parse them independently.
- Some lettre message-builder APIs may be unused; that is acceptable.
- Encoding edge cases (long headers, non-ASCII filenames, empty bodies) require explicit tests.
- The chosen boundary is documented in P07-001 and reviewed at P07-005.

## Evidence required

- Conformance tests parsing emitted MIME with `mail-parser`.
- Golden fixtures for text-only, HTML-only, alternative, mixed, and attachments.
- Header folding and RFC 2047/2231 tests.
- Wire-size parity between accounting and actual serialized bytes.
