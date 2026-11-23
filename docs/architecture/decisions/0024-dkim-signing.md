# ADR-0024 — DKIM signing

**Status:** ACCEPTED (project owner decision, 2026-11-23)
**Date:** 2026-11-23
**Related:** [ADR-0010](0010-mime-composition.md), [ADR-0013](0013-error-taxonomy.md), ADR-0017, [public API](../public-api.md), [error model](../error-model.md), [dependency policy](../../engineering/dependency-policy.md)

## Context

`0.1.0` through `1.1.1` send unsigned messages. Behind an ESP that signs, that is
invisible. For a team that relays through its own MTA, or tries to pass DMARC
through a plain SMTP hop, the client is the only place a signature can be added —
and every comparable client already does it: Nodemailer, Upyo and the hosted SDKs
all sign. The [competitive audit](../../research/competitive-landscape.md) ranked
DKIM as the second-largest gap after OAuth.

Two things Sendust should not become: a DNS client with a key store, and a
framework for every signature algorithm.

## Decision

Support **optional DKIM signing** (RFC 6376), driven by a caller-supplied key.

- **Shape.** `dkim: { domain, selector, privateKey, headers? }`, additive and
  optional. Without it a message is composed and sent exactly as before.
- **Where.** The core signs the composed wire bytes inside the delivery path,
  after `mime::serialize_at` and before the port is called. The transport keeps
  sending `DeliveryRequest::wire` unchanged, so the signed bytes and the bytes on
  the socket stay the same thing.
- **Algorithm.** `rsa-sha256` with relaxed/relaxed canonicalization for both the
  header block and the body, which is what receivers and ESPs accept. `rsa-sha1`
  is excluded (RFC 8301 deprecates it for signing).
- **Coverage.** The body is hashed in full — no `l=` tag, so appended content
  breaks the signature. `h=` lists the signed fields; `From` is always signed, a
  field the message does not carry is skipped rather than invented, and a field
  RFC 6376 forbids signing (`Bcc`, `Received`, `Return-Path`, `DKIM-Signature`,
  …) is rejected with `VALIDATION_INVALID_HEADER`.
- **Timestamps.** `t=` is the same instant that `Date` and `Message-ID` already
  carry, so the signature attests the headers the message actually has. There is
  no `x=` expiry: a hand-off queue is the caller's business, not the library's.
- **Key handling.** The PEM key (PKCS#1 or PKCS#8) is parsed when the transport is
  created, and a key outside 1024–4096 bits is refused — a signature that would
  need more than one header line is not one this client emits. An unusable key is
  `CONFIG_INVALID_DKIM_KEY` at construction, never a failure after `DATA`. The
  key is shared behind an `Arc`, redacted in `Debug`, and never rendered in an
  error or a log.
- **Size.** The wire cap covers the signature, because the cap is the bytes that
  reach the socket. A message that is exactly at the cap unsigned is
  `LIMIT_WIRE_BYTES` once signed.
- **Crypto.** Two new pinned dependencies: `rsa` 0.9.10 for RSASSA-PKCS1-v1_5 and
  `sha2` 0.10.9 for SHA-256, both RustCrypto, both `MIT OR Apache-2.0`. The
  canonicalization and the tag rendering are ours; the padding, the bignum
  arithmetic and the digest are not.
- **Out of scope.** DNS publication of the public key, key storage or rotation,
  Ed25519 (`ed25519-sha256`), multiple signatures per message, `i=`/`q=`/`l=`
  tags, and ARC. A caller who needs Ed25519 can add it behind the same option;
  nothing here assumes RSA at the contract level.

This is additive. No existing option, result, outcome, timeout, cap or default
changes, and one append-only error code is added. Under the
[versioning policy](../../engineering/versioning-policy.md) it is a minor release.

## Alternatives

1. **Sign with `mail-auth`.** Rejected as the signing path: it brings an async DNS
   stack and a second MIME parser into the core for two hundred lines of
   canonicalization and one PKCS#1 v1.5 signature check. It is a better verifier
   than signer, and verification belongs in tests.
2. **Sign in TypeScript.** Rejected: the wire bytes belong to the core (ADR-0010),
   and a signature computed behind the native boundary would duplicate
   composition and the wire cap.
3. **Sign in the SMTP adapter just before `DATA`.** Rejected: a session would
   re-sign per attempt, so the bytes in `DeliveryRequest` would no longer be the
   bytes sent, and a retry could attest a different `t=`.
4. **`rsa-sha1` or `l=` for compatibility.** Rejected: both are deprecated for
   signing, and `l=` lets a relay append content that the signature still covers
   only in part.
5. **Ship a key store, or generate keys.** Rejected: no credential store in a
   library that holds none, and no certificate authority in an email client.

## Consequences

- A team running its own MTA can sign from the client, which closes the last
  protocol gap between Sendust and the clients it is compared with.
- A signed message is roughly 0.4–0.7 KB larger for a 1024–2048-bit key; the
  default 30 MiB wire cap is unaffected in practice, and the cap check is exact.
- `CONFIG_INVALID_DKIM_KEY` is added to the error taxonomy, append-only.
- The signature covers `Date` and `Message-ID`, so the instant the signer is given
  is the instant in the message; `Delivery::send` passes one instant to both.
- Selector choice, DNS publication, rotation and revocation stay with the caller.
  A signature whose selector has no matching DNS record still reaches the relay
  and is still reported as `accepted`: relays accept by envelope, and deliverability
  is not observable from here. That limitation is stated in the option's docs.
- Ed25519 signing, ARC and multiple signatures remain open; the option is a
  struct, so adding an algorithm later is an additive change.
- ADR-0017 listed DKIM as explicitly out of MVP scope. This ADR supersedes that
  exclusion; the rest of ADR-0017 (own the SMTP engine, trusted TLS/I/O
  primitives) stands.

## Evidence

- `crates/sendust-core/tests/dkim.rs`: an independent verifier — its own relaxed
  canonicalization and the raw RSA verify — re-derives the body hash, rebuilds the
  signed header block from the emitted bytes and validates the signature for a
  plain message, a multipart message with an attachment, an empty body, a custom
  `h=` set and a different message. A malformed key is a stable error, not a
  panic.
- `crates/sendust-core/src/dkim/mod.rs`: RFC 6376 §3.4.2 and §3.4.4 canonicalization
  vectors, deterministic output for one instant, `From` always signed, absent
  fields skipped, forbidden fields rejected, key size and key encoding checks, and
  a `Debug` render that carries no key material.
- `crates/sendust-core/src/delivery/mod.rs`: the port receives a signed wire, and a
  signature that pushes the message over the wire cap never reaches the port.
- `ts/test/transport.test.ts`: a loopback relay receives a `DKIM-Signature` with the
  documented tags and header order before `DATA`, and an unusable key fails
  `createTransport` with `CONFIG_INVALID_DKIM_KEY`.
