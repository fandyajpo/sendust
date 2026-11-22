# Changelog

All notable changes to Sendust are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/). The rules — categories,
tag format, version sources, and compatibility promises — are in
[docs/engineering/versioning-policy.md](docs/engineering/versioning-policy.md).

## [Unreleased]

### Added

- `linux-x64-musl` and `linux-arm64-musl` native targets. The addon is built and
  loaded inside a musl container on the runner of that architecture, so Alpine
  and other musl images resolve a real package instead of failing closed with
  `CONFIG_NATIVE_UNAVAILABLE`. The loader, packaging and release manifest know
  the two keys; the artifacts ship with the next release, and until then the
  packages stay absent from the registry.

## [1.0.2] - 2026-11-21

A resource fix. No API, result, error code, option, or limit changed.

### Fixed

- An idle native session is released when `delivery.sessionIdleMs` elapses,
  instead of waiting for the next send or `close()`. A service that sends one
  message and stays up no longer holds an authenticated relay connection open.
- A transport with an invalid `delivery` option no longer creates a native
  transport before rejecting the option.
- The pool no longer re-pools a session that the close path already owns.

## [1.0.1] - 2026-11-21

A performance fix. No API, result, error code, option, or limit changed; what
changed is when the pool fills and how many round trips a message costs.

### Changed

- Concurrency rises to the work already queued instead of one slot per eight
  consecutive successes. A caller who hands Sendust a burst now uses the whole
  configured pool from the first message, which removes the queue wait that
  dominated p95 and p99 on a real relay. Throttling still halves the level and
  holds a cooldown after a temporary failure, so a relay that pushes back is
  still obeyed.
- `RSET` is sent only when an envelope was left open — every recipient refused,
  or `DATA` refused — instead of after every completed transaction, removing one
  round trip per message. A session that received `421` is never reused, because
  the relay is closing the channel.
- Analysis, the measurement that motivated it, and what is still unmeasured:
  [real-relay optimization](docs/qa/real-relay-optimization.md).

## [1.0.0] - 2026-11-19

The first stable release. It makes concurrent delivery the default: a queue, a
pool, adaptive concurrency, and bounded retries replace the single-send
behaviour of `0.1.0-rc.1`, and a transport reuses one authenticated SMTP session
across messages.

### Added

- Autonomous delivery engine in the TypeScript facade: a bounded queue, a pool of
  native transports, adaptive concurrency, an optional message rate, and bounded
  retries for transient failures (ADR-0021).
- SMTP session reuse: a transport keeps its authenticated session after a
  definitive outcome, resets it with `RSET`, and re-dials only on an error, an
  `unknown` disposition, the message cap, or the idle window (ADR-0022).
- `transport.stats`, `delivery.*` options, and the `LIMIT_QUEUE_FULL` code.
- A Nodemailer comparison benchmark (`npm run bench`) with results recorded in
  [docs/qa/benchmark-nodemailer.md](docs/qa/benchmark-nodemailer.md).

### Changed

- A concurrent `send()` is queued and scheduled instead of rejecting with
  `TRANSPORT_BUSY`. A retry is bounded by the message's total send budget.

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
