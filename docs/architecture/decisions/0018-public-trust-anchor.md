# ADR-0018 — Additive public trust anchor for TLS

**Status:** ACCEPTED (implements the P14-001 decision; verification is never disabled)  
**Date:** 2026-11-15  
**Supersedes:** nothing  
**Related:** [C-07](../../product/requirements.md), [ADR-0003](0003-tls-rustls-ring-webpki.md), [P11-002 TLS audit](../../security/tls-auth-audit.md)

## Context

The public API verified every relay against the bundled WebPKI roots and exposed no trust input. That kept C-07 trivially true, but it also meant a loopback TLS relay could never be trusted, so no JavaScript-level test could observe an `accepted` send. P14-001 and P14-004 need exactly that: a clean consumer sending one message to a local fake relay and observing relay acceptance.

The Rust core already had the primitive: `SmtpConfig::with_root_certificate_pem` adds anchors on top of the bundled roots and cannot remove or relax verification.

## Decision

Add an optional `tlsRootCertificate` string (one PEM bundle) to `TransportOptions`, passed through the binding to `with_root_certificate_pem`.

- It is **additive only**: the bundled roots are always loaded first, and there is no companion option that empties the store or turns verification off.
- A malformed PEM is rejected at construction with `VALIDATION_INVALID_OPTION`, before any connection.
- The option is a trust input, not a verification switch: an unknown relay still fails closed with `TLS_CERTIFICATE`.
- Hostname verification and the TLS 1.2 floor are unchanged.

## Alternatives

1. **Disable verification for tests.** Rejected: it would create exactly the insecure toggle C-07 forbids.
2. **Trust every local certificate automatically.** Rejected: it silently weakens the default for every consumer.
3. **Keep no anchor.** Rejected: it leaves the distribution evidence impossible and keeps R-04 open indefinitely.

## Consequences

- Loopback and private-CA relays become testable from JavaScript; the P14 matrix and the R-04 fixture are unblocked.
- Consumers with a private CA can use the public API rather than the Rust core only.
- The default remains trust-on-first-use-free: no anchor, no extra trust.
- The TLS audit, public API contract, and traceability matrix are updated with this option.
