# Changelog

All notable changes to Sendust are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/). The rules — categories,
tag format, version sources, and compatibility promises — are in
[docs/engineering/versioning-policy.md](docs/engineering/versioning-policy.md).

## [Unreleased]

Nothing yet.

## [1.3.0] - 2026-11-24

The fixes from the source audit of `1.2.1`, plus one additive state. A caller
who neither reads `transport.state` nor sets `delivery.sessionIdleMs: 0` sees no
API change; the wire changes only where the relay advertises `SIZE`.

### Added

- `transport.state` and `stats.state` report `failed` when the engine stops on a
  condition that cannot heal — rejected or unsupported credentials. Every later
  `send()` rejects with that same error and dials nothing, where it used to read
  `open` and then reject with `TRANSPORT_CLOSED`, so a health check keyed on the
  state reported healthy forever. There is no in-process reset: rotate the
  credential and build a new transport. `close()` still settles `closed`
  ([public API](docs/architecture/public-api.md) §7).
- `MAIL FROM` declares `SIZE=<n>` when the relay advertises `SIZE` (RFC 1870),
  and a message above the relay's own limit fails locally with
  `LIMIT_WIRE_BYTES` at `stage: "mail"` before the envelope opens — instead of
  being disconnected mid-`DATA` and reported as `unknown`.

### Fixed

- An `AUTH` line is no longer held to the 512-octet command line: a bearer token
  and a base64 challenge answer serialise up to the 1,228-octet floor RFC 4954
  requires of servers (12,288 here), so an XOAUTH2 token from Microsoft 365 or
  Google Workspace works. A value over that bound is `VALIDATION_INVALID_OPTION`,
  not `VALIDATION_HEADER_INJECTION`, which accused the caller of injecting a
  command.
- A send whose spawned task ended abnormally no longer answers
  `TRANSPORT_BUSY` for the life of the process: it reports `INTERNAL`, and the
  transport is `failed` rather than looking merely busy.
- A pooled session is released exactly once when the native result fails
  mapping; a second release could hand one session to two sends at once.
- A session is reused only after the relay answers the `RSET` that closes the
  transaction with a positive reply; any other reply drops it, as ADR-0022 says.
- A cancelled send no longer reports the previous message's recipients as
  `unknown`: cancellation progress is reset before composition starts.
- `delivery.sessionIdleMs: 0` means no idle reuse on both sides: the session is
  closed when its message settles, which the core already did, instead of being
  pooled and then refused on reuse.
- `host` rejects embedded whitespace with `CONFIG_INVALID_RELAY`, as the
  documented contract says, instead of failing later as a connection reset.

### Changed

- CI runs the native binding on every claimed Node.js version — 22.23.5, 24.22.3
  and 26.13.1 — instead of one, and `tsc` type-checks the whole test suite
  rather than two of seven files, so a wrong type in the largest test file no
  longer reaches `main`. `@types/node` is pinned to `22.20.5`, the newest 22.x,
  so a test cannot type-check against an API the newest supported runtime lacks.
- The audit that found all of this is committed under `audit/code-audit/`: six
  documents, two HIGH, 13 MEDIUM, 15 LOW and 10 INFORMATIONAL findings, from a
  static review of the same tree.

### Verification

- Rust workspace: `cargo test --workspace --all-targets`, `cargo clippy
  --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`.
- TypeScript: `tsc` over the whole suite, Biome lint and format, and 79 Vitest
  tests, including the new pool-release and failed-state proofs.
- `npm run check`: architecture, links, doc claims and version agreement.

## [1.2.1] - 2026-11-24

A metadata patch for the npm page. No runtime, API, result, error code, limit or
default changed; the code and the wire are byte-identical to `1.2.0`.

### Changed

- The published manifest declares `homepage` (the documentation site) and `bugs`
  (the issue tracker), and carries the keywords npm search needs. `1.2.0` shipped
  without them, so its page linked only to the repository and could not be found
  by protocol or capability.
- `ts/README.md`, which *is* the npm page, now reads as one: install, a quick
  start, both authentication modes, DKIM, the engine's guarantees, the seven
  targets, runtime requirements and links into the documentation.
- The seven platform packages copy `homepage`, `bugs` and `keywords` from the
  facade, so their pages carry the same project links.

## [1.2.0] - 2026-11-23

Signing, not a new transport. Optional and additive: a caller who does not set
`dkim` sends byte-identical messages to `1.1.1`. One append-only error code is
added.

### Added

- Optional DKIM signing (RFC 6376), so a team that relays through its own MTA can
  pass DMARC without an ESP that signs: `dkim: { domain, selector, privateKey }`.
  The core signs the composed wire — `rsa-sha256`, relaxed/relaxed — and prepends
  one `DKIM-Signature` before `DATA`, so the bytes the transport sends are the
  bytes that were signed. `From` is always covered, an absent field is skipped, a
  field RFC 6376 forbids signing is rejected, and the body is hashed in full (no
  `l=`). The key is parsed when the transport is created, so an unusable key fails
  with `CONFIG_INVALID_DKIM_KEY` instead of after the relay has seen a message,
  and the wire cap covers the signature
  ([ADR-0024](docs/architecture/decisions/0024-dkim-signing.md)).

- Two pinned dependencies for it, `rsa` 0.9.10 and `sha2` 0.10.9, both RustCrypto
  and `MIT OR Apache-2.0` ([dependency policy](docs/engineering/dependency-policy.md)).

## [1.1.1] - 2026-11-23

A metadata patch, published to validate npm trusted publishing. No runtime,
API, result, error code, limit or default changed.

### Changed

- The published manifests declare `repository.url`, which npm trusted publishing
  requires to match the GitHub repository for every package it publishes.

## [1.1.0] - 2026-11-22

Reach, not features: the two things that stopped a real team from using Sendust
— a missing Alpine build and no OAuth 2.0 authentication. No result, error code,
limit or default changed for existing callers.

### Added

- `AUTH XOAUTH2` (RFC 7628) with a caller-supplied bearer token, so tenants that
  disable password authentication — Microsoft 365, Google Workspace — can use
  Sendust: `auth: { user, accessToken, method: "xoauth2" }`. The token may be a
  string or a synchronous function read once per session. XOAUTH2 is never
  chosen implicitly, the `334` error challenge is answered properly, and the
  token is redacted exactly like a password
  ([ADR-0023](docs/architecture/decisions/0023-xoauth2-authentication.md)).

- `linux-x64-musl` and `linux-arm64-musl` native targets. The addon is built and
  loaded inside a musl container on the runner of that architecture, so Alpine
  and other musl images resolve a real package instead of failing closed with
  `CONFIG_NATIVE_UNAVAILABLE`. Both targets build, load and pass the behaviour
  suite in CI; the artifacts ship with the next release, and until then the
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
