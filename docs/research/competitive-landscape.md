# Competitive landscape

**Date:** 2026-11-21.
**Subject:** `@sendust/ts` 1.0.2 against Nodemailer, Upyo, EmailJS, and the
hosted-API SDKs.
**Method:** repository facts for Sendust; npm registry metadata, npm download
counts (week 2026-11-14 → 2026-11-20), and each project's own documentation for
the rest. Every outside number is dated and linked; nothing here is estimated.

This is not a delivery claim and not a market forecast. It is what Sendust is,
what it is not, and where it currently loses.

## The 30-second answer

Sendust and Nodemailer do the same job: they are SMTP clients you point at a
relay you already have. Sendust's difference is that the SMTP engine, the MIME
composer, and the queue are native Rust behind a narrow typed API, with hard
caps, mandatory TLS verification, and a result model that says `unknown` when
the relay's answer was lost — and never retries that. Nodemailer has 25 years of
battle testing and every feature; Sendust has a smaller, stricter surface and
roughly 2.2–3.7× the throughput on the project's own loopback benchmark.

Upyo is the closest strategic competitor: a cross-runtime TypeScript library
with SMTP pooling, retries, DKIM, OAuth2 and ten provider transports, MIT
licensed, still pre-1.0. It is broader than Sendust and runs where Sendust
cannot (browser, edge).

EmailJS is not a competitor at all: it is a hosted template service with a
quota and a dashboard. Neither is Resend, SendGrid, Postmark or SES — those are
the relays. Sendust is the client on the other side of them.

## 1. What Sendust is

A native SMTP delivery engine for Node.js, Bun, and Deno: two Rust crates
(`sendust-core`, `sendust-napi`) behind a TypeScript facade, shipped as five
prebuilt platform packages ([ADR-0019](../architecture/decisions/0019-installable-typescript-package.md),
[ADR-0020](../architecture/decisions/0020-per-platform-native-packages.md)).

What it actually guarantees, from the code rather than the README:

- **Transport:** SMTP over implicit TLS or STARTTLS, certificates and hostname
  always verified, no plaintext mode and no switch to turn verification off.
- **Auth:** `PLAIN` and `LOGIN` only.
- **Message:** From, To, CC, BCC, Reply-To, Subject, text, HTML, custom headers,
  buffer attachments, opt-in local-file attachments. MIME composed in Rust.
- **Result:** `accepted`, `rejected`, or `unknown`, with per-recipient statuses
  and the final SMTP response when one was seen. `unknown` is never retried.
- **Bounds:** 100 recipients, 10 MiB per attachment, 20 MiB total, 30 MiB wire;
  timeouts 10/30/120/125 s that may only be lowered; queue 10 000 messages or
  64 MiB with `LIMIT_QUEUE_FULL`; concurrency ≤ 32; 3 attempts by default.
- **Sessions:** one SMTP transaction per session at a time, one authenticated
  session reused for up to 100 messages or 30 s idle, then released.
- **Runtimes:** Node.js ≥ 22.23.5, Bun 1.4.4, Deno 2.9.8 (FFI, local
  `node_modules`). Platforms: `linux-x64-gnu`, `linux-arm64-gnu`,
  `macos-x64`, `macos-arm64`, `windows-x64`, plus `linux-x64-musl` and
  `linux-arm64-musl`, which are built and tested in CI and published with the
  next release.

## 2. Feature comparison

Legend: ✅ first-class, ◐ partial or via a plugin, ❌ absent.

| Capability | Sendust 1.0.2 | Nodemailer 10.6.1 | Upyo 0.6.1 | EmailJS | Resend 6.39.0 |
| --- | --- | --- | --- | --- | --- |
| SMTP transport | ✅ | ✅ | ✅ | ❌ (their API) | ❌ (their API) |
| HTTP provider APIs | ❌ | ◐ community plugins | ✅ 10+ providers | ✅ (only theirs) | ✅ (only theirs) |
| Runs in browser / edge | ❌ | ❌ | ✅ | ✅ | ✅ |
| Deno / Bun support | ✅ | ◐ Bun tests in repo | ✅ | ✅ | ✅ |
| Prebuilt native binary | ✅ | ❌ pure JS | ❌ pure TS | ❌ | ❌ |
| Connection pooling | ✅ bounded | ✅ | ✅ | n/a | n/a |
| Bounded retries | ✅ | ❌ caller's job | ✅ `RetryTransport` | n/a | n/a |
| First-class `unknown` outcome | ✅ | ❌ | ◐ structured errors | ❌ | ❌ |
| Hard caps on recipients/size | ✅ | ❌ | ◐ size bounds | ✅ plan quotas | ✅ plan quotas |
| TLS verification cannot be disabled | ✅ | ❌ (`tls.rejectUnauthorized`) | ◐ per-transport options | n/a | n/a |
| DKIM signing | ✅ (1.2.0) | ✅ | ✅ rsa + ed25519 | ◐ provider-side | ◐ provider-side |
| OAuth2 / XOAUTH2 | ❌ | ✅ | ✅ | n/a | n/a |
| Templates | ❌ by design | ❌ (plugins) | ❌ | ✅ dashboard | ✅ React Email |
| Tracking, webhooks, analytics | ❌ by design | ❌ | ◐ provider passthrough | ✅ | ✅ |
| `AbortSignal` cancellation | ❌ | ❌ | ✅ | ❌ | ❌ |
| SMTPUTF8 | ❌ | ◐ | ✅ | n/a | n/a |
| npm provenance attestation | ❌ | ✅ | ❌ | ❌ | ✅ |
| Weekly npm downloads | 457 | 40 467 423 | 5 184 | 8 300 | 18 474 069 |

Sources: npm registry metadata for each package; `upyo.org/why` for Upyo's own
comparison; EmailJS pricing page for plan limits. Download counts are
`api.npmjs.org/downloads/point/last-week`, week 2026-11-14 → 2026-11-20.

## 3. Nodemailer — the incumbent

**What it is:** the default SMTP client for Node.js since 2010. Pure
JavaScript, `MIT-0`, `node >= 20`, 165 files and 1.8 MB unpacked, published with
provenance from GitHub Actions. Transports: SMTP, SES, stream, JSON, sendmail.
Pooling, DKIM, OAuth2, `verify()`, plugins for everything else.

**Where Sendust differs, concretely:**

- **The engine is not JavaScript.** SMTP parsing, MIME encoding and the queue
  run in Rust, off the event loop. The project's loopback benchmark measured
  2.2× (sequential) to 3.7× (1000 messages at concurrency 32) Nodemailer's
  throughput with a lower p95 ([benchmark](../qa/benchmark-nodemailer.md)).
- **The result model is explicit.** Nodemailer reports `info` and raises an
  error when the connection dies; Sendust returns `unknown` when the final
  reply after `DATA` was lost, and refuses to retry it. That distinction is the
  difference between a duplicate invoice and a possibly-lost one.
- **Security defaults have no escape hatch.** Nodemailer accepts
  `tls: { rejectUnauthorized: false }`; Sendust has no such option, by
  [ADR-0003](../architecture/decisions/0003-tls-rustls-ring-webpki.md).
- **Everything is bounded.** Caps, timeouts, queue depth, queue bytes, pool
  size and session lifetime are documented numbers with stable error codes;
  Nodemailer leaves those to the caller and the relay.
- **Install is a native binary.** 42 files, ~151 kB unpacked, no postinstall
  download, no compiler.

**Where Nodemailer wins:** DKIM, OAuth2, SMTPUTF8, plugin ecosystem, 40 million
weekly installs, 16 years of edge cases in the protocol, and it is pure JS, so
there is no platform matrix to fall off.

**Honest caveat.** On the owner's real Gmail run against `1.0.0`, Sendust was
about 7 % slower in throughput with roughly twice Nodemailer's p95 (11.9 s vs
5.3 s); the `1.0.1` fixes target exactly that tail, and have not been
re-measured yet ([analysis](../qa/real-relay-optimization.md)). The loopback win
is real but it is a CPU-bound comparison, not a provider comparison.

## 4. Upyo — the closest strategic competitor

**What it is:** a cross-runtime TypeScript email library by Hong Minhee, MIT,
`0.6.1`, pre-1.0, 589 GitHub stars, ~5.2 k weekly downloads, zero runtime
dependencies, published to npm and JSR. Same idea as Nodemailer — write a
message once, pick a transport — but with web-standard primitives and no native
code.

**What it has that Sendust does not:**

- **Ten provider transports** (Resend, SendGrid, Mailgun US/EU, SES v2 with
  SigV4, Mailtrap, Maileroo, Mailtrap, Plunk, Lettermint, Cloudflare) plus
  JMAP. One message type, many transports, chosen by environment.
- **Edge and browser support** through `fetch()`; SMTP itself is Node/Bun/Deno
  only, which they say plainly.
- **DKIM** (`rsa-sha256` and `ed25519-sha256`, multiple signatures), **OAuth2**
  (XOAUTH2, OAUTHBEARER), **SMTPUTF8**, DSN, `sendRaw()`, `verify()`,
  idempotency keys, calendar invitations, `AbortSignal`, batch `sendMany()`.
- **`RetryTransport`** with full-jitter exponential backoff, provider
  `Retry-After` honouring, cancellations never retried, and a `PoolTransport`
  with weighted round-robin and failover.
- **Decorators a team can drop in:** mock transport, LogTape logging,
  OpenTelemetry traces.

**What Sendust has that Upyo does not:**

- **The protocol is not TypeScript.** Upyo's SMTP work runs on the JS event
  loop; Sendust's runs natively, and the loopback benchmark puts that at
  2.2–3.7× Nodemailer — Upyo does not publish a comparable measurement.
- **A contract rather than an API.** `unknown`, `LIMIT_QUEUE_FULL`,
  `TRANSPORT_CLOSED`, `CONFIG_NATIVE_UNAVAILABLE` are stable, documented codes
  with a stated retry rule; Upyo's errors are structured but provider-shaped.
- **One transport, bounded.** No provider matrix to choose from, so there is
  nothing to get wrong: the queue, the pool and the session lifetime are fixed
  policy with numbers in the docs.
- **A versioned release gate.** Every release is approved, hashed and
  re-verified from the registry; Upyo publishes 0.x nightlies (166 versions).

**Verdict.** Upyo is broader and better packaged for the modern serverless
world; Sendust is narrower, stricter, and native. Anyone who needs Resend or
SendGrid as an HTTP API, edge functions, DKIM or OAuth2 should use Upyo today.

## 5. EmailJS — a different product

EmailJS is a **hosted template mailer**: templates live in a dashboard, the
browser or server calls their API with a template id and variables, and their
servers do the SMTP work. The official Node SDK is `@emailjs/nodejs` 5.0.2
(BSD-3-Clause, `node >= 18`, ~8 300 weekly downloads).

Plans are quota-based: 200 requests/month free, `$9` for 2 000, `$15` for
5 000–10 000, `$40` for 25 000–200 000, with template and attachment ceilings
per tier. Emails carry an "EmailJS" footer until a paid tier removes it, and
history retention is 7 days on free ([pricing](https://www.emailjs.com/pricing/)).

There is no overlap with Sendust's job: you cannot bring your own relay, choose
your TLS roots, see an SMTP reply code, or control the queue. The trade is
convenience for control, and the cost is per-message.

## 6. The hosted relay SDKs — Resend, SendGrid, Postmark, SES

These are not competitors either: they are the relays Sendust sends *to*. Their
SDKs are HTTP clients for their own API.

| SDK | Version | Weekly downloads | Licence | Notes |
| --- | --- | ---: | --- | --- |
| `resend` | 6.39.0 | 18 474 069 | MIT | React Email templates, provenance attested, `node >= 20` |
| `@sendgrid/mail` | 8.x | 6 957 293 | MIT | Largest feature surface, provider-side templates and tracking |
| `postmark` | 4.x | 1 723 609 | MIT | Transactional-first, strict message streams |
| `@aws-sdk/client-sesv2` | 3.x | 10 267 201 | Apache-2.0 | IAM/SigV4, the default inside AWS |
| `@emailjs/nodejs` | 5.0.2 | 8 300 | BSD-3-Clause | Template service, see §5 |

The practical consequence: a team using Resend or SendGrid over HTTP does not
need Sendust, and a team using any of them over SMTP can point Sendust at their
SMTP endpoint instead. Sendust's honest pitch is "your relay, but with a native
engine and a real contract", not "a better provider".

## 7. Where Sendust genuinely wins

1. **A result model nobody else has.** `unknown` after `DATA`, never retried,
   with per-recipient detail — the only honest answer when the relay's last
   reply was lost.
2. **Security posture a reviewer can sign off.** Verification cannot be
   disabled, credentials never appear in errors, filesystem attachments are
   opt-in, every buffer is owned and every cap has a number.
3. **Bounded by construction.** Queue depth, queue bytes, pool size, concurrency
   and timeouts are policy, not tuning advice, and each violation has a stable
   code.
4. **Native throughput off the event loop**, measured at 2.2–3.7× Nodemailer on
   one machine, with the caveat in §3.
5. **Three runtimes, one artifact set.** Node, Bun and Deno through the same
   addon; no compiler, no postinstall, no per-runtime build.
6. **A release process that can be audited.** Approved commit, pinned hashes,
   registry re-verification ([publication checklist](../engineering/publication-checklist.md)).

## 8. Where Sendust loses, ranked

1. **Alpine / musl Linux was unsupported.** `linux-x64-gnu` and
   `linux-arm64-gnu` only, so `node:alpine` failed closed with
   `CONFIG_NATIVE_UNAVAILABLE`. Both musl targets now build, load and pass their
   suite in a musl container as of 2026-11-22; the packages ship with the next
   release and are absent from the registry until then.
2. **No OAuth2.** Tenants that disable basic auth (Microsoft 365, Google
   Workspace with modern policy) cannot use Sendust at all, because `PLAIN` and
   `LOGIN` are the only mechanisms.
3. **No DKIM signing.** Fine behind an ESP that signs; a blocker for teams
   running their own MTA that expect the client to sign. **Closed in 1.2.0**
   ([ADR-0024](../architecture/decisions/0024-dkim-signing.md)): `rsa-sha256`
   relaxed/relaxed signing with a caller-supplied key. Ed25519 is still absent,
   and so are multiple signatures.
4. **No HTTP provider transports and no edge support.** Every serverless or
   edge deployment is out of reach by construction.
5. **Adoption and trust.** 457 weekly downloads against Nodemailer's 40.5
   million; the first stable release is days old. Downloads at this size also
   include CI traffic, so it is a rounding error, not a signal.
6. **No npm provenance attestation.** Nodemailer and Resend publish with SLSA
   attestations; `@sendust/ts` publishes with a bypass-2FA token and no
   attestation, which is the one place where the supply-chain story is weaker
   than the code.
7. **No `AbortSignal`, no SMTPUTF8, no streaming attachments, no mock
   transport** for consumers writing tests.
8. **Platform breadth.** No Windows arm64, no musl, and Node.js ≥ 22.23.5 is a
   high floor.

## 9. What to do next, in order

| Priority | Move | Why |
| --- | --- | --- |
| 1 | Publish `linux-x64-musl` and `linux-arm64-musl` builds | Removes the most common install failure (Alpine containers) — code and CI landed 2026-11-22, awaiting the first green run |
| 2 | Add XOAUTH2 and DKIM | Unlocks Microsoft 365 / Google Workspace tenants and self-hosted-MTA users — **shipped: XOAUTH2 in 1.1.0, DKIM in 1.2.0** |
| 3 | Move to npm trusted publishing (OIDC) with provenance | Closes the provenance gap and the January 2027 token deadline |
| 4 | Ship a mock transport | Test ergonomics; upyo treats it as table stakes |
| 5 | `AbortSignal` mapped to bounded close | Already a post-MVP candidate; callers expect it |
| 6 | Publish this comparison on the docs site | Buyers compare; own the honest version of the table |

Deliberately not on the list: HTTP provider transports, templates, tracking and
a hosted service. They would turn Sendust into a different product with a
different support burden, and
[vision](../product/vision.md) rules them out.

## 10. Sources and how to re-check

| Claim | Source |
| --- | --- |
| Sendust limits, defaults, codes | `ts/src/limits.ts`, `ts/src/transport.ts`, [error model](../architecture/error-model.md) |
| Sendust throughput | [benchmark-nodemailer.md](../qa/benchmark-nodemailer.md), `npm run bench` |
| Sendust real-relay latency | [real-relay-optimization.md](../qa/real-relay-optimization.md) |
| Nodemailer version, licence, transports | `https://registry.npmjs.org/nodemailer/latest` |
| Upyo features, runtimes, transports | `https://upyo.org/why`, `https://github.com/dahlia/upyo` |
| EmailJS plans and limits | `https://www.emailjs.com/pricing/` |
| Download counts | `https://api.npmjs.org/downloads/point/last-week/<package>` |
| Provenance attestations | the `attestations` field in each package's registry metadata |

To refresh the numbers, fetch the downloads endpoint and the registry metadata
again; the comparison table in §2 is the only part that needs editing.
