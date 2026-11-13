# ADR-0003 — rustls with ring and webpki-roots; verification mandatory

**Status:** ACCEPTED (architecture baseline; source-reviewed feature graph); compile and fixture validation required before implementation  
**Date:** 2026-11-13  
**Deciders:** Research gate (P01-005), security review (P11-002)  
**Validated by:** P01-002, P06-002, P11-002

## Context

Prebuilt native binaries must have consistent TLS behavior across macOS, Linux, and Windows. Native TLS (OpenSSL/Schannel/Secure Transport) varies by platform, complicates cross-compilation, and can produce different trust behavior per host. rustls is pure Rust and is supported by lettre 0.11.23.

lettre’s observed documentation requires selecting a rustls crypto provider and a certificate verification backend:

- Provider: `ring`, `aws-lc-rs`, or a custom provider via `rustls-no-provider`.
- Verification backend: `rustls-platform-verifier`, `rustls-native-certs`, or `webpki-roots`.

## Decision

Use rustls 0.23 with the `ring` provider and `webpki-roots` trust anchors. Certificate and hostname verification are always enabled. Sendust exposes no public option, environment variable, or undocumented flag to disable verification. lettre's builder contains dangerous certificate/hostname bypass methods; Sendust MUST NOT call them.

Rationale for `webpki-roots`: deterministic trust across all shipped binaries, no dependence on the consumer OS trust store, no platform certificate loading behavior to validate. Rationale for `ring`: broad platform support and simpler cross-compilation than `aws-lc-rs`.

## Alternatives

1. **`rustls-platform-verifier`.** Uses the OS trust store; better for enterprises with private CAs. More platform-specific behavior and deployment-target requirements to validate.
2. **`rustls-native-certs`.** Loads OS roots; intermediate between deterministic and platform-integrated behavior.
3. **`native-tls`.** Platform TLS; adds OpenSSL/Schannel dependencies and cross-build complexity.
4. **`aws-lc-rs` provider.** Stronger assembly-backed performance, but heavier build requirements.

## Consequences

- Trust behavior is identical on every platform and reproducible in CI.
- Enterprise users with private CAs cannot rely on OS-installed roots in the MVP; this must be documented as a limitation and considered for post-MVP.
- `ring` and rustls feature names may drift; the exact feature set must be pinned and validated.
- Verification failures must be tested with five fixtures: self-signed, expired, wrong host, incomplete chain, malformed certificate.

## Source review and implementation gate

The pinned v0.11.23 Cargo.toml exposes a composable feature graph: `default-features = false`, with `smtp-transport`, `tokio1-rustls`, `ring`, and `webpki-roots`. This avoids lettre defaults including `pool` and `native-tls`. Source inspection is complete; the exact Cargo build and target compilation remain hard gates at P03-002/P06-002. No claim of compiled validation is made here.

## Evidence required

- A feature matrix naming exact features and the source proving each (recorded in [lettre validation](../../research/lettre-0.11.23-validation.md)).
- A compiled dependency graph with no duplicate/conflicting rustls providers.
- Negative certificate tests failing closed.
- A source audit showing no call to `dangerous_accept_invalid_certs(true)` or `dangerous_accept_invalid_hostnames(true)`, and no bypass in the Sendust public API.
