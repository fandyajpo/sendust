# Sendust — MVP Scope

**Status:** proposed.  
**Authoritative feature classification:** [feature matrix](../planning/feature-matrix.md).  
**Authoritative constraints:** [requirements](requirements.md).  
**This document summarizes; it does not override either.**

## 1. MVP definition

Sendust MVP is a production-usable npm package that sends one transactional email per call through a consumer-configured SMTP relay, with correct MIME composition, mandatory TLS verification, structured outcomes, bounded resources, and prebuilt native binaries for a validated platform matrix.

## 2. Included capabilities

### SMTP

- Implicit TLS and STARTTLS.
- PLAIN and LOGIN authentication.
- Connect, operation, total, and close timeouts.
- SMTP response classification.
- Relay-accepted semantics.
- One connection per send; deterministic close.

### Message composition

- From with optional display name.
- To, CC, BCC, Reply-To.
- Subject with RFC 2047 encoding.
- Plain text and HTML bodies.
- `multipart/alternative` and `multipart/mixed`.
- Validated custom headers.
- Correct CRLF and header folding.

### Attachments

- In-memory buffers.
- Opt-in local regular files.
- Explicit or inferred content type.
- Safe filename encoding.
- Hard per-file, aggregate, and wire caps.

### Delivery

- Single message per call.
- Up to 100 total recipients.
- Structured `accepted` / `rejected` / `unknown` result.
- Honest recipient status per ADR-0005.
- Immediate `BUSY` on concurrent send.
- Idempotent bounded close.

### Developer experience

- TypeScript declarations.
- ESM and CommonJS.
- Stable error classes and codes.
- npm installation without a Rust toolchain.
- Quick start, API, limits, errors, troubleshooting docs.
- Runnable examples using a local fake relay.

### Packaging and quality

- Root package plus platform optional packages.
- Validated OS/CPU/libc and Node.js matrix.
- CI tests, artifact provenance, SBOM, checksums.
- Security and performance evidence.

## 3. Excluded from MVP

| Capability | Classification | Rationale |
| --- | --- | --- |
| Connection pooling | POST-MVP | Adds lifecycle, health-check, and concurrency complexity; MVP is one send per transport |
| Automatic retry/backoff | POST-MVP | Retrying after DATA can duplicate mail; caller owns retry |
| SMTPUTF8 | POST-MVP | Requires relay capability negotiation and broader address model |
| AbortSignal cancellation | POST-MVP | Bounded close and timeouts cover MVP shutdown; adds a second cancellation path |
| Streaming attachments | POST-MVP | Conflicts with wire-size accounting and bounded memory simplicity |
| URL attachment fetch | OUT OF SCOPE | SSRF, network, and trust surface |
| DKIM signing | POST-MVP | Key management and canonicalization scope |
| Additional transports (SES, SendGrid, Mailgun, sendmail) | POST-MVP | MVP is SMTP-first |
| Templates | OUT OF SCOPE | Application concern |
| Queue/job integration or adapters | HELD | Not part of the current product roadmap; no queue API, worker, persistence, or adapter contract will be designed in this phase |
| Open/click tracking | OUT OF SCOPE | SaaS concern |
| Analytics dashboard | OUT OF SCOPE | SaaS concern |
| Contact/campaign management | OUT OF SCOPE | SaaS concern |
| Web administration | OUT OF SCOPE | SaaS concern |
| Provider REST APIs | OUT OF SCOPE | Not SMTP |
| IMAP/POP3 receiving | OUT OF SCOPE | Delivery library only |
| `verify()` connection probe | POST-MVP | Useful but not required to send |
| Proxy support | POST-MVP | Not required by the MVP use case |
| Multiple concurrent sends per transport | OUT OF SCOPE | Use multiple transports |
| Plaintext SMTP | OUT OF SCOPE | Security posture requires TLS |
| Certificate verification bypass | OUT OF SCOPE (permanent) | Security posture |

## 4. MVP success conditions

The MVP is complete only when:

1. All 18 delivery phases (P00–P17) have approved gates.
2. The [feature matrix](../planning/feature-matrix.md) shows every REQUIRED row with completed implementation and test tasks.
3. All [quality gates](../planning/quality-gates.md) pass with linked evidence.
4. A clean install on every supported target sends through a fake relay and closes.
5. Published metadata, platform packages, provenance, and documentation are internally consistent.
6. No excluded capability appears in the public API, docs, or tests as if supported.

## 5. Scope-change rule

Any addition requires:

1. A requirement ID in [requirements.md](requirements.md).
2. A classification change in the feature matrix.
3. A task, a test, and an owner in a phase plan.
4. Review at the next phase gate.

Features cannot enter the MVP through implementation first and documentation later. Anything discovered mid-implementation that is not in this scope is recorded as post-MVP unless it is required for correctness or security; security fixes are always in scope.

## 6. Known limitations to document at release

- One active send per transport; no internal queue, worker, scheduler, or queue adapter. Queueing and scheduling remain caller-owned.
- No automatic retry; `unknown` requires caller judgment.
- No delivery confirmation.
- No SMTPUTF8; ASCII local parts only.
- No streaming; whole message buffered within caps.
- Filesystem attachments disabled by default.
- No plaintext SMTP.
- Support matrix limited to tested targets.
