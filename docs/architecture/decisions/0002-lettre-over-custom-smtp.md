# ADR-0002 — Use lettre 0.11.23 instead of a custom SMTP client

**Status:** ACCEPTED (architecture baseline); lower-level async API selected by P01 source review; behavior validation P06  
**Date:** 2026-11-13  
**Deciders:** Research gate (P01-005)  
**Validated by:** P01-001, P01-003, P06-001

## Context

Sendust’s core value is a reliable SMTP engine with correct TLS, authentication, MIME, and response handling. Implementing SMTP, STARTTLS, AUTH, MIME encoding, and reply parsing from scratch is a large correctness and security surface. `lettre 0.11.23` was observed on docs.rs (published 2026-10-10, MIT) with async Tokio support, TLS feature selection, a typed message builder, and structured errors.

The main uncertainty is observability: a high-level transport may hide per-recipient RCPT outcomes and exact stage boundaries. Sendust must not promise capabilities the library does not expose.

## Decision

Use `lettre 0.11.23` as the SMTP and message-primitive dependency. Pin the exact version. Use the public lower-level `AsyncSmtpConnection` API because `command(Rcpt::new(...))` exposes individual RCPT responses and `Error::status()` exposes numeric negative reply codes. The lower-level API still delegates SMTP parsing, command formatting, TLS, AUTH, and DATA dot-stuffing to lettre. Keep Sendust's orchestration limited to public connection/command methods.

Do not implement a custom SMTP client in the MVP.

## Alternatives

1. **Custom SMTP client on Tokio.** Maximum control over RCPT status and stage timing, but requires implementing greetings, EHLO, STARTTLS, AUTH mechanisms, dot-stuffing, line limits, reply parsing, and a large fault matrix. High risk and maintenance burden.
2. **`mail-send`.** Simpler, but less complete async/TLS/MIME integration; would require more MIME work and has a smaller ecosystem.
3. **Shell out to a system `sendmail`.** Not portable, not SMTP-first, and introduces process and environment dependencies.

## Consequences

- SMTP correctness and TLS integration benefit from an established implementation.
- Sendust inherits lettre’s feature-flag complexity and must pin and audit it.
- Per-recipient RCPT responses are observable through the selected lower-level API; the selected behavior is recorded in ADR-0005.
- Upgrading lettre is a deliberate, tested change, not a floating range.
- The public contract must be expressed in Sendust’s own result/error types, not lettre’s, so the dependency can be replaced if necessary.

## Conditions that would reverse this decision

Source/build validation demonstrates that:

- A required MVP behavior (mandatory TLS verification with the chosen backend, STARTTLS, PLAIN/LOGIN auth, single DATA transaction for the accepted-recipient subset, bounded close) cannot be implemented through public lettre APIs; or
- The selected API is unmaintained or has an unresolved security advisory.

Reversal requires a superseding ADR and an updated Phase 06 plan before implementation.

## Evidence required

- Pinned dependency entry with exact version and enabled features.
- Capability matrix citing the public API used for each behavior.
- Fault-matrix tests exercising the chosen API.
- No hand-written SMTP command strings in the codebase.
