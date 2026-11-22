# ADR-0023 — XOAUTH2 authentication

**Status:** ACCEPTED (project owner decision, 2026-11-22)
**Date:** 2026-11-22
**Related:** [ADR-0003](0003-tls-rustls-ring-webpki.md), ADR-0017, [public API](../public-api.md), [error model](../error-model.md)

## Context

`0.1.0` through `1.0.2` authenticate with `AUTH PLAIN` and `AUTH LOGIN` only.
Both need a password. Microsoft 365 and Google Workspace tenants increasingly
disable basic authentication and require OAuth 2.0, so a consumer on those
tenants cannot use Sendust at all — not because of a feature gap in the sending
path, but because there is no way to present a token.

Two things Sendust should not become: an OAuth client (authorization endpoints,
refresh tokens, a token store) and a credential cache. Both belong to the
application that already owns the identity provider.

## Decision

Support **`AUTH XOAUTH2`** (RFC 7628), driven by a caller-supplied bearer token.

- **Mechanism.** The client sends `AUTH XOAUTH2 <base64>`, where the payload is
  `user=<mailbox>` `\x01` `auth=Bearer <token>` `\x01` `\x01`.
- **Challenge.** If the relay answers `334`, the body is a base64 JSON error
  description. The client does not parse it: it answers with an empty line so the
  relay sends the final failure reply, and that reply becomes the error.
- **Selection.** XOAUTH2 is never chosen implicitly. A token is not a password,
  so the caller has to ask for the mechanism with `auth.method: "xoauth2"`, and
  an unadvertised `XOAUTH2` in `EHLO` fails with `AUTH_UNSUPPORTED` instead of
  falling back to `PLAIN`.
- **Secrets.** The token lives in the same secret slot as a password: it is
  validated as non-empty and control-character free, redacted in `Debug`, and
  never rendered in an error or a log.
- **Refresh.** The facade accepts `auth.accessToken` as a string or as a
  synchronous function. The function is read once per native session — when the
  transport is built and whenever the pool grows — so a long-lived transport can
  hand each new session a fresh token. Async refresh, refresh-token storage and
  the OAuth flow stay with the caller, who can rebuild the transport when a
  token expires.
- **Public shape.** `auth: { user, accessToken, method?: "xoauth2" }`, exactly one
  of `pass` or `accessToken`; anything else is `VALIDATION_INVALID_OPTION`.

This is additive: `auth: { user, pass, method? }` behaves as before, and no
result, error code, timeout, cap or default changes. Under the
[versioning policy](../../engineering/versioning-policy.md) it is a minor
release.

## Alternatives

1. **An OAuth client inside Sendust.** Rejected: it would need an authorization
   endpoint, a browser or device flow, a token store, and per-provider client
   registration — a product of its own, and a credential store in a library that
   currently holds none.
2. **Let the caller inject an `Authorization` header.** There is no such header
   in SMTP; authentication is a SASL exchange on the session.
3. **Auto-select XOAUTH2 when `XOAUTH2` is advertised.** Rejected: the relay
   advertising the mechanism says nothing about whether the caller has a token,
   and silently preferring it would send a password as a bearer token.
4. **Parse the `334` JSON.** Rejected: the payload is provider-specific error
   text; the final reply code is the stable signal the library already models.

## Consequences

- A team on an OAuth-only tenant can use Sendust by obtaining a token from its
  own identity provider and passing it in — no protocol work on their side.
- `AUTH_UNSUPPORTED` now also covers "you asked for XOAUTH2 but the relay does
  not advertise it".
- The e-mail account's password is no longer the only accepted secret, which is
  an improvement for tenants that forbid passwords entirely.
- Token lifetime is the caller's problem by design; a session created with an
  expired token fails with `AUTH_FAILED`, and the engine does not retry it
  (rejected credentials are fatal by policy).

## Evidence

- `crates/sendust-core/tests/smtp_session.rs`: XOAUTH2 authenticates against a
  relay that advertises it, the payload decodes to the RFC 7628 string, a `334`
  challenge produces the final `535` as `AUTH_FAILED` without leaking the token,
  and an unadvertised mechanism is `AUTH_UNSUPPORTED` with no `AUTH` on the wire.
- `ts/test/transport.test.ts`: a transport authenticates with a token callback,
  the callback is read once for one session, and the invalid `auth`
  combinations are rejected with `VALIDATION_INVALID_OPTION`.
