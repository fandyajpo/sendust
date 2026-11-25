# ADR-0026 — Populate the documented error details; deprecate `PROTOCOL_REJECTED`

**Status:** ACCEPTED (project owner decision 2026-11-25; details additive, one code deprecated)  
**Date:** 2026-11-25  
**Deciders:** Project owner, from the source audit of 1.2.1 (M-10, M-11)

## Context

`ErrorDetails` documented `recipient` and `smtpEnhancedCode`, and no path
produced them. A `535 5.7.8` reached a caller as `smtpCode: 535` alone, and an
unexpected `RCPT` reply lost the address it was about.

`PROTOCOL_REJECTED` was documented with no path that produced it. Error codes
are append-only within a major version, so it cannot be deleted here.

## Decision

- Populate `recipient` in the `RCPT` loop, where one address owns the reply,
  and `smtpEnhancedCode` wherever a reply carried an RFC 3463 code: at AUTH and
  at the transaction's own stages.
- Keep `PROTOCOL_REJECTED` in the catalogue and mark it deprecated: deprecated
  in 1.4.0, removed in the next major.

## Alternatives

1. **Delete the two details.** Cheaper, but they are exactly the facts a caller
   debugging an auth failure needs, and the data is present when the error is
   built.
2. **Delete `PROTOCOL_REJECTED` now.** It would break the append-only promise
   inside a major version.
3. **Find a path for `PROTOCOL_REJECTED`.** No failure needs it: a definitive
   refusal is a `rejected` outcome, and a malformed reply is
   `PROTOCOL_UNEXPECTED_RESPONSE`.

## Consequences

- `details` carries only what the relay sent: a reply without an enhanced code
  adds nothing, so the record cannot be mistaken for a fabricated one.
- Two documented fields stop being unproducible, so the error model and the
  guides describe the adapter rather than a wish.
- `PROTOCOL_REJECTED` is documented as deprecated rather than produced, and the
  next major may delete it.

## Evidence

- `crates/sendust-core/tests/smtp_faults.rs`:
  `auth_failure_is_an_error_and_never_mails` asserts `5.7.8`, and
  `an_intermediate_rcpt_reply_is_a_protocol_error` asserts both the recipient
  and the enhanced code.
- `docs/architecture/error-model.md` and
  `docs/guides/errors-security-operations.md` carry the deprecation.
