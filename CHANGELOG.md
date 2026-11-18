# Changelog

All notable changes to Sendust are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/). The rules — categories,
tag format, version sources, and compatibility promises — are in
[docs/engineering/versioning-policy.md](docs/engineering/versioning-policy.md).

## [Unreleased]

Nothing yet. The next candidate is `0.1.0-rc.1` once P16 finishes assembling it.

## [0.1.0-rc.1] - 2026-11-18

The first MVP release candidate. Not published; publication is P17.

### Added

- Native SMTP delivery engine in Rust: implicit TLS and STARTTLS with `rustls`
  (`ring`, `webpki-roots`), AUTH `PLAIN`/`LOGIN`, per-recipient `RCPT` outcomes,
  one `DATA` transaction per accepted subset, and explicit final-reply
  classification.
- Deterministic message composition: validated addresses (ASCII local parts,
  IDNA domains, Unicode display names), reserved-header protection, `Date` and
  `Message-ID`, `text`/`html` alternatives, RFC 2231 filenames, and an exact
  wire-size cap.
- Hard limits: 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw,
  30 MiB wire; timeouts of 10 s connect, 30 s operation, 120 s total, 125 s
  close, all finite and lower-only.
- Explicit lifecycle: `open -> closing -> closed`, one active send per transport,
  immediate `TRANSPORT_BUSY`, idempotent bounded close, no GC-dependent
  shutdown, no retry, no queue, no pool.
- Node-API 8 binding with owned copies before async work, plus a runtime-neutral
  TypeScript facade for Node.js, Bun, and Deno with a narrow public surface and
  stable error codes.
- Distribution: five native targets — `linux-x64-gnu`, `linux-arm64-gnu`,
  `macos-arm64`, `macos-x64`, `windows-x64` — packaged as per-platform
  `@sendust/ts-<platform>-<arch>` optional dependencies selected by npm, and
  validated per target on Node.js, Bun, and Deno.
- Documentation: quick start, lifecycle and limits, addresses/MIME/attachments,
  errors/security/operations, and maintainer guides, with an executed quick
  start sample and a claims checker.

### Security

- TLS verification cannot be disabled; there is no plaintext mode. The only
  trust input is the additive `tlsRootCertificate` (ADR-0018).
- Error payloads, messages, and logs exclude credentials, bodies, attachment
  bytes, and absolute paths; a redaction regression is a security failure.
- Filesystem attachments are opt-in and limited to bounded regular files; no
  URL or remote fetch form exists.

### Known limitations

- No mailbox delivery confirmation: `accepted` is relay acceptance.
- Post-DATA ambiguity is `unknown` and is not retried automatically.
- SMTPUTF8, connection pooling, internal queues, provider-specific transports,
  and private-CA support beyond the add-trust-anchor option are out of scope.
- macOS x64, Windows arm64, and musl targets are unsupported and absent from
  publication inputs.
