# Sendust — Product Vision

**Standalone Native Email Delivery Engine.**  
Rust core. Shared Node-API native addon. Runtime-neutral TypeScript API. SMTP-first.

## 1. Problem

Server-side JavaScript teams using Node.js, Bun, or Deno need to send transactional email through a configured SMTP relay. The dominant experience, Nodemailer, is mature but entirely JavaScript: MIME composition, SMTP session handling, encoding, and connection management all run on the event loop and are maintained as a large, historically grown surface.

Sendust exists to offer a smaller, stricter alternative:

- Native Rust implementation of SMTP, MIME, and validation.
- A deliberately narrow TypeScript API.
- Honest delivery semantics.
- Bounded resource use and explicit lifecycle.
- Prebuilt binaries so consumers never need a Rust toolchain.

Sendust is inspired by Nodemailer’s developer experience. It is **not** a clone, a drop-in replacement, or a Rust port of Nodemailer’s feature set.

## 2. Target users

| Persona | Context | What they need |
| --- | --- | --- |
| Backend application developer | Node.js service sending password resets, receipts, notifications through a configured relay | A small API, TypeScript types, reliable behavior, no native build step |
| Platform engineer | Operates the relay and the Node fleet | Predictable TLS/auth behavior, bounded memory, explicit shutdown, no hidden retries |
| Library maintainer | Publishes the npm package | A testable Rust core, deterministic packaging, a support matrix that matches CI |
| Security reviewer | Reviews dependencies and data flow | No credential leakage, no verification bypass, bounded input handling |

Non-personas for MVP: marketing operators, campaign managers, mailbox hosting administrators, and end users reading email.

## 3. Product principles

1. **Relay acceptance is not delivery.** A successful send means the configured SMTP relay accepted responsibility for the message. Sendust never claims mailbox delivery.
2. **One obvious path.** One transport, one active send, one explicit close. No internal queue, worker, background delivery, automatic retry, or connection pool in the current scope.
3. **Correct over convenient.** RFC-compliant headers, correct MIME boundaries, deterministic encoding, and strict input validation beat permissive guessing.
4. **Bounded by construction.** Every buffer, timeout, recipient list, attachment, and close drain has a finite, documented limit.
5. **Secure defaults, no escape hatch.** Certificate verification cannot be disabled. Filesystem attachment access is opt-in. Credentials are redacted from errors.
6. **Native without pain.** npm installation selects a prebuilt binary. No postinstall download, no compiler requirement for consumers.
7. **Evidence over assertion.** A phase is complete only with executable tests, artifacts, or reviewed records.
8. **Modular without fragmentation.** Keep one product and one shared native engine, with strict internal module ownership and dependency boundaries. Do not split into crates, services, or plugins without evidence.

## 4. Product scope

### MVP

- SMTP over TLS (implicit TLS) and STARTTLS, with certificate verification.
- PLAIN and LOGIN authentication against a relay.
- One message per `send` call: From, To, CC, BCC, Reply-To, Subject, text, HTML, custom headers.
- Buffer and opt-in local-file attachments.
- MIME multipart composition with correct transfer encoding.
- Structured results and stable error classes.
- TypeScript declarations, ESM and CommonJS loading.
- Prebuilt native packages for a validated platform matrix.

### Post-MVP candidates

Connection pooling, carefully scoped retry research, SMTPUTF8, AbortSignal-based cancellation, streaming attachments, additional transports, DKIM, and observability hooks may be evaluated separately. Queue/job orchestration and queue adapters are **held outside the current product roadmap**: no API, adapter contract, persistence model, worker, or implementation is being designed. See [MVP scope](mvp-scope.md) and the [feature matrix](../planning/feature-matrix.md).

### Explicit non-goals

Sendust is a standalone delivery engine, not an email service platform. Campaign management, contact management, analytics, open/click tracking, email hosting, web administration, built-in template management, and provider-specific REST integrations are outside the product. Queueing, scheduling, and retry orchestration are caller-owned; queue adapters and internal queue infrastructure are not part of the current roadmap.

## 5. Success criteria for the MVP

The MVP is successful when all of the following are true and evidenced:

1. A clean project on every supported platform installs the published package from npm and sends a message through a local fake SMTP relay without a Rust toolchain.
2. Every MVP feature in the [feature matrix](../planning/feature-matrix.md) maps to at least one completed task and at least one test.
3. SMTP, MIME, error, lifecycle, security, and packaging quality gates in [quality-gates.md](../planning/quality-gates.md) pass.
4. The support matrix states exact Node.js, Bun, and Deno versions plus OS/CPU/libc targets, Node-API level, and what CI actually tests.
5. No credential, body content, attachment bytes, or absolute local path leaks into serialized errors.
6. The package publishes with provenance, complete platform artifacts, and a verified clean-install path.

## 6. Positioning

| Dimension | Sendust | Nodemailer | Raw SMTP crate in Node |
| --- | --- | --- | --- |
| Runtime | Rust core + Node-API addon for Node.js, Bun, Deno | JavaScript | Varies |
| API surface | Narrow, typed, one transport | Broad, plugin-oriented | Low-level, untyped |
| Delivery semantics | Explicit relay acceptance | Transport-dependent | Caller-defined |
| Native install | Prebuilt platform packages | Pure JS | Requires toolchain or download |
| MVP scope | Transactional SMTP | Very broad | Minimal |

Sendust competes on correctness, predictability, and a small supported surface — not on feature count.

### Runtime support

Node.js, Bun, and Deno are required MVP runtime targets under [ADR-0015](../architecture/decisions/0015-multi-runtime-support.md). This is a product target, not a current compatibility claim. Every runtime/platform combination must pass Sendust-owned blocking CI before it is listed as supported. Deno requires local node_modules and explicit FFI permission.


