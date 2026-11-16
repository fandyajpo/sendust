# TLS and Authentication Posture Audit (P11-002)

**Status:** audited — no verification bypass exists  
**Date:** 2026-11-15  
**Scope:** the native client's rustls configuration, the public and binding surfaces that could influence it, and the authentication exchange and its error paths.

## 1. TLS configuration review

| Check | Result | Evidence |
| --- | --- | --- |
| Trust store is the bundled Mozilla roots | PASS | `tls.rs`: `RootCertStore::empty()` plus `webpki_roots::TLS_SERVER_ROOTS` |
| Caller-supplied anchors only add trust; they cannot replace or empty the store | PASS | `client_config` seeds webpki roots first, then adds `config.root_certificates()` |
| Hostname verification is on | PASS | `ServerName::try_from(host)` feeds the rustls connector; no verifier override |
| TLS 1.2 is the floor, safe defaults for versions and ciphers | PASS | `with_safe_default_protocol_versions()` with the `ring` provider and the `tls12` feature |
| No client certificate auth | PASS | `with_no_client_auth()` |
| No dangerous switch exists in the code | PASS | `check-architecture` fails on `danger_accept_invalid_certs`, `danger_accept_invalid_hostnames`, `dangerous(`, `set_certificate_verifier`, `insecure_skip_verify`, `disable_verification` |
| No alternative TLS backend is a direct dependency | PASS | `check-architecture` forbids `native-tls`, `openssl`, and `rustls-native-certs` in the core |

**Public surface**: `secure` accepts only `tls` and `starttls` — there is no plaintext mode; the transport options expose no verification flag.

- The Node product surface adds **one additive trust anchor**: `tlsRootCertificate` (a PEM string) was added in [ADR-0018](../architecture/decisions/0018-public-trust-anchor.md) for private-CA and loopback test relays. It cannot empty or replace the bundled roots, has no companion way to disable verification, and a malformed anchor is `VALIDATION_INVALID_OPTION`. `TransportOptions` in `ts/src/types.ts` otherwise carries only `host`, `port`, `secure`, `helloName`, `auth`, `timeouts`, `limits`, `attachments`, and that anchor.
- The Rust core API does expose `SmtpConfig::with_root_certificate_pem` (re-exported with `SmtpConfig` from `lib.rs`). It is additive by construction: `client_config` seeds the bundled WebPKI roots first and then appends caller anchors, and `RootCertStore::add` rejects a malformed anchor. No argument can empty or replace the store, and there is no companion method that removes verification. The Rust example and the test harness use it for a local CA; the signing keys are not in the repository.

**STARTTLS**: a configured `starttls` that the relay does not advertise fails with `TLS_HANDSHAKE` before `MAIL`; there is no opportunistic downgrade path.

## 2. Certificate failure matrix

| Fixture | Result |
| --- | --- |
| Self-signed | `TLS_CERTIFICATE`, no `MAIL` |
| Expired | `TLS_CERTIFICATE`, no `MAIL` |
| Wrong host | `TLS_CERTIFICATE`, no `MAIL` |
| Incomplete chain | `TLS_CERTIFICATE`, no `MAIL` |
| Untrusted root | `TLS_CERTIFICATE`, no `MAIL` |
| Malformed anchor | `TLS_HANDSHAKE` when configuring |
| Malformed server certificate | handshake failure, `TLS_HANDSHAKE` |

Evidence: `every_bad_certificate_fixture_fails_before_mail`, `an_untrusted_certificate_fails_closed`, `fails_closed_for_each_bad_certificate_fixture`, `rejects_a_malformed_trust_anchor`, and the repeated-failure leak probe.

## 3. Authentication review

| Check | Result | Evidence |
| --- | --- | --- |
| Only PLAIN and LOGIN are implemented | PASS | `AuthMechanism` enum; `choose_auth` |
| AUTH is refused before TLS | PASS | `auth_before_tls_is_refused` → `AUTH_UNSUPPORTED` |
| A relay with no supported mechanism is rejected without sending credentials | PASS | `auth_without_a_supported_mechanism_is_unsupported` / `auth_without_a_mechanism_is_unsupported` |
| A forced mechanism must be advertised | PASS | `an_explicit_mechanism_must_be_advertised` |
| `535` is a terminal `AUTH_FAILED` with the SMTP code, no `MAIL` or `DATA` | PASS | `auth_failure_is_an_error_and_never_mails` |
| The error carries neither the password nor the username in Display or Debug | PASS | Same test now renders `{error:?} {error}` and asserts both markers are absent |
| `SmtpCredentials` Debug redacts both fields | PASS | `credentials_are_redacted_and_validated` |
| Credentials are never logged or traced | PASS | No logging crate; the only trace is the fake harness's test artifact |

## 4. Feature and dependency review

- The workspace pins `rustls =0.23.46` with `default-features = false` and exactly `std`, `tls12`, `ring`.
- `tokio-rustls` is pinned with the same `tls12` feature and no default features.
- `webpki-roots =1.0.12` supplies the trust store; no OS certificate loader is present.
- No `native-tls`, `openssl`, `openssl-sys`, or `webpki-roots`-alternative is a direct dependency, now enforced by `check-architecture`.
- There is no feature flag, environment variable, or method that disables verification.

## 5. Residual risks

| Risk | Decision |
| --- | --- |
| Private enterprise roots installed only in the OS are not trusted | Accepted MVP trade-off (threat-model R-01); callers with a private CA use the core anchor path or a relay with a public certificate |
| The core anchor API (`with_root_certificate_pem`) adds trust by design | Accepted; not exposed through the binding or the facade |
| A compromised host process can patch the binary | Out of scope; the library is not a sandbox |

## 6. Findings

No high-severity finding. One low note: the `ring` provider is installed through rustls' builder rather than a process-wide `install_default`, which is deliberate — it keeps the crypto provider local to this client and avoids a global side effect. P11-005 will re-run this audit with the gate evidence.
