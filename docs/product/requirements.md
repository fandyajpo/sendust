# Sendust — Requirements

**Status:** authoritative for MVP scope and constraints.  
**Research date:** 2026-11-13.  
**Related:** [vision](vision.md), [MVP scope](mvp-scope.md), [feature matrix](../planning/feature-matrix.md), [dependency research](../engineering/dependency-research.md).

This document defines **what must be true** for the MVP. It does not describe implementation order; that is in the [phase plans](../phases/). Where a requirement is unresolved, the owning validation task is named and the requirement is marked **UNRESOLVED**. No implementation may assume the favorable answer.

## 1. Interpretation

- **MUST** — mandatory for MVP. Absence blocks release.
- **MUST NOT** — prohibited.
- **SHOULD** — expected; deviation requires a recorded reason in the relevant ADR or gate record.
- **MAY** — optional; never a completion condition.
- A requirement is satisfied only when its enforcing task has evidence accepted at its phase gate.
- Requirements are referenced by ID from phase plans and architecture documents; they are not redefined elsewhere.

## 2. Constraints

### C-01 — Exactly two Rust crates

The workspace contains exactly two crates:

- `crates/sendust-core` — validation, message model, MIME, attachment policy, transport state, SMTP orchestration, errors. It MUST NOT depend on Node, `napi`, or JavaScript runtime types.
- `crates/sendust-napi` — Node-API adapter only. It MUST NOT contain SMTP or MIME business logic.

Enforced by P02-001, P03-001, P04-005, P05-005.

### C-02 — Dependency direction

The runtime path is `TypeScript wrapper (ts/src) -> napi binding -> sendust-core -> lettre`. Dependencies MUST NOT point upward. The TypeScript wrapper MUST NOT implement SMTP, MIME, or validation policy beyond public-boundary shape checks.

Enforced by P02-001, P03-001, P09-001.

### C-03 — SMTP engine

`lettre 0.11.23` is the observed SMTP engine at planning time. Sendust MUST use lettre rather than a custom SMTP client unless P01-001/P01-003 prove a required MVP capability is inaccessible through public APIs. Any replacement requires an ADR approved before Phase 06.

Enforced by P01-001, P01-005, P06-001.

### C-04 — Async runtime

The MVP MUST use Tokio 1 through the runtime managed by napi-rs. It MUST NOT create a second runtime. Blocking filesystem reads MUST NOT run on the async reactor thread.

Enforced by P01-004, P05-003, P08-002.

### C-05 — Node-API level

The binding targets **Node-API 8** as the approved design floor. The initial pins are napi 3.16.2, napi-derive 3.7.0, and napi-build 2.6.0; the napi crate MUST enable the `napi8` and `async` features explicitly because its default ABI feature is napi4. P01-004 records the source research; compilation and runtime compatibility MUST pass at P05-003/P14-004. A higher level requires a superseding ADR and support-matrix update.

Enforced by P01-004, P05-001, P14-002.

### C-06 — Node.js support claims

Node.js 22, 24, and 26 are **test candidates**, not support claims. Only versions with blocking CI runtime tests may be declared supported. Every claim MUST name exact tested versions.

Enforced by P01-004, P14-004, P15-001.

### C-07 — TLS verification

TLS MUST use rustls with the `ring` provider and `webpki-roots` trust anchors, with the source-reviewed lettre feature graph in ADR-0003. Cargo compilation and certificate fixtures remain hard implementation gates. Certificate and hostname verification MUST be always on. No public option, environment variable, or undocumented flag may disable verification. TLS 1.2 is the minimum.

Enforced by P01-002, P06-002, P11-002.

### C-08 — Single active send

Each transport permits at most one active `send`. A concurrent call MUST fail immediately with `BUSY`. It MUST NOT be queued, delayed, or silently dropped.

Enforced by P02-003, P04-004, P09-004, P10-001.

### C-09 — Explicit lifecycle

Lifecycle is `open -> closing -> closed`. `close()` MUST be explicit, idempotent, deadline-bounded (default 125 s), and MUST NOT depend on garbage collection, finalizers, or process exit. Sends during `closing` or after `closed` MUST fail with a stable error.

Enforced by P02-003, P04-004, P09-004, P10-001.

### C-10 — Timeout budgets

Defaults and maximums: connect 10 s, per-operation 30 s, total send 120 s, close/drain 125 s. Values MAY be lowered but MUST NOT exceed these maxima. Each value MUST be at least 1 s and finite. Connect and operation budgets MUST NOT exceed total; close/drain MUST be at least total and no more than 125 s. Zero, negative, sub-second, `NaN`, and infinite values MUST be rejected. Stage attribution is limited to coarse public API operations; internal socket sub-stage timing is not promised.

Enforced by P01-003, P02-003, P04-004, P10-002.

### C-11 — Owned memory

JavaScript-owned `Buffer`, `ArrayBuffer`, typed array, and string data MUST be copied into Rust-owned memory before any `await`. No borrowed JavaScript memory may cross an async boundary. Attachment file bytes MUST be read into bounded Rust-owned memory.

Enforced by P05-002, P08-002, P08-004, P11-004.

### C-12 — Attachment filesystem policy

Filesystem attachments MUST be opt-in and disabled by default. When enabled, only bounded regular files may be read. Symlinks, directories, FIFOs, devices, and sockets MUST be rejected. URL, remote, stream, directory, and glob attachment forms MUST NOT exist. Absolute paths MUST NOT appear in public errors.

Enforced by P02-004, P08-001, P08-002, P11-003.

### C-13 — Hard caps

| Limit | Value |
| --- | --- |
| Recipients per message (To + CC + BCC) | 100 |
| Per attachment raw bytes | 10 MiB |
| Aggregate raw attachment bytes | 20 MiB |
| Serialized wire message | 30 MiB |
| Active sends per transport | 1 |

Exceeding a cap MUST fail before DATA. Caps MUST NOT be advisory, silently truncated, or raisable above the documented maximum.

Enforced by P02-004, P04-003, P08-003, P09-001.

### C-14 — Address and Unicode policy

Envelope local parts MUST be ASCII. Domains MUST be IDNA-converted for SMTP. Display names, subjects, and bodies MAY be Unicode and MUST be encoded per MIME rules. SMTPUTF8 is post-MVP and MUST be rejected or documented as unsupported, never silently mishandled.

Enforced by P04-002, P07-001, P07-004.

### C-15 — Delivery semantics

A successful result means the relay accepted the message for at least one accepted recipient. It NEVER means mailbox delivery. An explicit RCPT refusal rejects only that recipient when other recipients were accepted; if all RCPT commands are refused, the overall result is `rejected`. An explicit negative final DATA reply is also `rejected`. After message transmission begins, an indeterminate final disposition MUST be reported as `unknown` and MUST NOT be retried automatically.

Enforced by P02-002, P06-004, P09-002.

### C-16 — No pooling, no automatic retry, no queue orchestration

MVP MUST open one connection per send and close it deterministically. Sendust MUST NOT include a connection pool, automatic retry policy, backoff scheduler, internal queue, worker, persistence layer, dead-letter queue, or queue adapter. Each `send()` call is a direct request/response operation. Queueing, scheduling, and any retry decision remain caller-owned and may be implemented by the consuming application independently of Sendust. No queue abstraction or adapter contract is to be designed in the current roadmap.

Enforced by P02-003, P09-004, and the product-scope review.

### C-17 — Cancellation

AbortSignal support is optional and MUST NOT be required for MVP. Bounded close and timeout budgets are required. Any later AbortSignal implementation MUST map onto the same bounded-close semantics and MUST NOT introduce a second cancellation path.

Enforced by P02-003, P09-004, P18-002.

### C-18 — Recipient-level SMTP outcomes

Sendust MUST observe each RCPT reply through lettre's public lower-level async connection API. A 2xx RCPT reply is accepted at the envelope stage; an ordinary 4xx/5xx reply is a definitive rejection for that recipient. A 421 reply closes the session, so Sendust MUST stop recipient collection and MUST NOT issue DATA even if earlier RCPT replies were 2xx. Otherwise, if at least one recipient is accepted, Sendust MUST submit exactly one DATA transaction to the accepted-recipient subset. A recipient is `accepted` only after positive final DATA. If final DATA is explicitly refused, those accepted-RCPT recipients become `rejected`; if final disposition is unknown, they become `unknown`. Sendust MUST NOT send independent per-recipient messages.

Enforced by P01-003, P02-002, P06-003, P09-002.

### C-19 — Redaction

Public errors, logs, and diagnostics MUST NOT contain SMTP passwords, authentication tokens, full message bodies, attachment bytes, or unredacted absolute local paths. Host and port MAY appear. Usernames SHOULD be redacted or omitted.

Enforced by P02-002, P09-003, P11-004.

### C-20 — Stable error taxonomy

Every public error MUST have a stable machine-readable `code`, a stable class/category, a human-readable message, and optional structured details. Callers MUST NOT need to parse message strings. Rust panics MUST NOT cross the Node-API boundary; expected failures use `Result`.

Enforced by P04-001, P05-004, P09-003.

### C-21 — Deterministic test environment

All automated tests MUST use local, deterministic fake SMTP/TLS fixtures. No test may contact a real relay or external network, and no credentials may be committed. Fault injection MUST be explicit and repeatable.

Enforced by P00-004, P03-004, P12-004.

### C-22 — Module formats

The package MUST support ESM `import` and CommonJS `require` for the public API, with identical runtime behavior and TypeScript types. If a format cannot be supported on a target, the limitation MUST be documented and gated.

Enforced by P09-001, P14-002, P14-004.

### C-23 — Reproducible, traceable release

Published artifacts MUST be built by CI from a tagged commit with locked dependencies, include provenance, and match the platform packages listed by the root package. Partial publication MUST have a documented recovery procedure.

Enforced by P16-002, P16-004, P17-001, P17-004.

### C-24 — Evidence-based completion

No task or phase may be marked `COMPLETED` without the evidence named in its plan and the project-wide [definition of done](../planning/definition-of-done.md). Planning documents are not evidence of implemented behavior.

Enforced by every phase gate.

### C-25 — Single source of truth

Requirements live here; task detail lives in phase plans; feature classification lives in the feature matrix; ADRs live in `architecture/decisions/`. Other documents MUST link rather than duplicate.

Enforced by P15-005 and documentation review.

### C-26 — Honest support matrix

Support claims MUST match what CI actually tests. The matrix MUST name Node-API level, exact tested Node.js/Bun/Deno versions, OS/CPU/libc targets, minimum OS/glibc floor, and the tested runtime × platform combinations. “All platforms” and “all runtime versions” are prohibited.

Enforced by P14-003, P14-004, P15-001.

### C-27 — No consumer toolchain

Consumers MUST install prebuilt binaries from npm without a Rust toolchain or a postinstall binary download.

Enforced by P14-002, P14-004, P17-004.

### C-28 — No plaintext SMTP

Supported security modes are implicit TLS and STARTTLS only. Plaintext SMTP MUST NOT be implemented.

Enforced by P06-002, P11-002.

### C-29 — Injection prevention

CRLF and control characters MUST be rejected in addresses, header names, header values, filenames, and the EHLO identity. Reserved headers MUST NOT be overridable through custom headers.

Enforced by P04-002, P07-001, P11-003.

### C-30 — Standalone delivery-engine boundary

Sendust is a standalone native email delivery engine, not a hosted email platform or job-processing system. Marketing, campaigns, contacts, analytics, tracking, hosting, web administration, built-in templates, provider REST APIs, and inbound mail protocols are outside the product. Queue/job orchestration is held outside the current roadmap: no internal queue, worker, persistence, scheduling, or adapter API is defined. Reconsideration requires a separate product decision and updated requirements; it is not implied by the post-MVP phase.

Enforced by P00-001 and P15-005.

## 3. Functional requirements

### 3.1 SMTP

| ID | Requirement | Enforcement | Tests |
| --- | --- | --- | --- |
| FR-001 | Configure relay host, port, security mode, EHLO identity, and optional authentication. | P06-001, P09-001 | P06-005, P12-002 |
| FR-002 | Establish implicit TLS connections. | P06-001, P06-002 | P06-005 |
| FR-003 | Upgrade connections with STARTTLS. | P06-002 | P06-005 |
| FR-004 | Authenticate with PLAIN. | P06-002 | P06-005 |
| FR-005 | Authenticate with LOGIN. | P06-002 | P06-005 |
| FR-006 | Enforce a connect timeout. | P04-004, P10-002 | P10-004 |
| FR-007 | Enforce a per-operation timeout. | P04-004, P10-002 | P10-004 |
| FR-008 | Enforce a total send timeout. | P04-004, P10-002 | P10-004 |
| FR-009 | Parse and classify SMTP responses into outcomes and errors. | P06-004 | P06-005 |
| FR-010 | Verify certificates and hostnames on every TLS connection. | P06-002 | P06-005, P11-002 |
| FR-011 | Reject unsupported security modes and authentication mechanisms at configuration time. | P06-002 | P06-005 |

### 3.2 Message composition

| ID | Requirement | Enforcement | Tests |
| --- | --- | --- | --- |
| FR-020 | Set From with an optional display name. | P07-001 | P07-005 |
| FR-021 | Set To recipients. | P07-001, P06-001 | P07-005, P06-005 |
| FR-022 | Set CC recipients. | P07-001, P06-001 | P07-005, P06-005 |
| FR-023 | Set BCC recipients without exposing them in headers. | P07-001, P06-001 | P07-005, P06-005 |
| FR-024 | Set Reply-To. | P07-001 | P07-005 |
| FR-025 | Set a Unicode-capable Subject encoded per RFC 2047. | P07-001 | P07-005 |
| FR-026 | Provide a plain-text body. | P07-002 | P07-005 |
| FR-027 | Provide an HTML body. | P07-002 | P07-005 |
| FR-028 | Add validated custom headers. | P07-001 | P07-005, P11-003 |
| FR-029 | Compose `multipart/alternative` when both text and HTML are present. | P07-002, P07-004 | P07-005 |
| FR-030 | Compose `multipart/mixed` when attachments are present. | P07-002, P08-004 | P07-005, P08-005 |
| FR-031 | Validate addresses: ASCII local part, IDNA domain, optional Unicode display name. | P04-002, P07-001 | P04-005, P07-005 |
| FR-032 | Reject CRLF and control characters in addresses, headers, filenames, and EHLO identity. | P04-002, P07-001 | P07-005, P11-003 |
| FR-033 | Emit CRLF line endings and legal header folding. | P07-001, P07-004 | P07-005 |
| FR-034 | Reject overrides of reserved headers. | P07-001 | P07-005 |
| FR-035 | Encode non-ASCII display names and subjects per RFC 2047. | P07-001 | P07-005 |

### 3.3 Attachments

| ID | Requirement | Enforcement | Tests |
| --- | --- | --- | --- |
| FR-040 | Attach in-memory buffers. | P08-004 | P08-005 |
| FR-041 | Attach local files only when filesystem access is explicitly enabled. | P08-001, P08-002 | P08-005 |
| FR-042 | Read only bounded regular files. | P08-002 | P08-005, P11-003 |
| FR-043 | Encode attachment filenames safely. | P07-003 | P07-005, P08-005 |
| FR-044 | Resolve content type explicitly or by safe extension mapping, defaulting to `application/octet-stream`. | P07-003 | P07-005, P08-005 |
| FR-045 | Enforce per-file, aggregate, and wire caps before DATA. | P08-003 | P08-005, P13-003 |
| FR-046 | Reject URL, stream, directory, and glob attachments. | P08-001 | P08-005, P11-003 |

### 3.4 Delivery and lifecycle

| ID | Requirement | Enforcement | Tests |
| --- | --- | --- | --- |
| FR-050 | Send exactly one message per call. | P09-002 | P12-003 |
| FR-051 | Support up to 100 total recipients. | P06-001, P07-001 | P06-005, P12-002 |
| FR-052 | Return a structured result with `accepted`, `rejected`, or `unknown` and recipient statuses. | P02-002, P09-002 | P09-005, P12-003 |
| FR-053 | Report recipient-level failure honestly per ADR-0005. | P06-003, P09-002 | P06-005, P12-002 |
| FR-054 | Reject concurrent sends with `BUSY` and no I/O. | P04-004, P09-004 | P10-004, P12-003 |
| FR-055 | Close explicitly and idempotently within the close budget. | P09-004, P10-001 | P10-004, P12-003 |
| FR-056 | Reject sends after close with a stable error. | P04-004, P09-004 | P10-004 |
| FR-057 | Never retry automatically and never send per-recipient messages. | P06-003, P06-004 | P06-005 |

### 3.5 Developer experience

| ID | Requirement | Enforcement | Tests |
| --- | --- | --- | --- |
| FR-060 | Ship TypeScript declarations. | P05-001, P09-001 | P09-005, P12-003 |
| FR-061 | Support ESM `import`. | P14-002 | P14-004 |
| FR-062 | Support CommonJS `require`. | P14-002 | P14-004 |
| FR-063 | Install from npm without a Rust toolchain. | P14-004 | P17-004 |
| FR-064 | Expose stable error classes and codes. | P09-003 | P09-005, P12-003 |
| FR-065 | Provide runnable examples against a fake relay. | P09-005, P15-001 | P15-005 |
| FR-066 | Provide quick start, API, limits, errors, security, and troubleshooting documentation. | P15-001–P15-005 | P15-005 |
| FR-067 | Reject unknown option keys. | P09-001 | P12-003 |
| FR-068 | Export the package version. | P09-001 | P12-003 |

## 4. Non-functional requirements

| ID | Requirement | Enforcement | Tests |
| --- | --- | --- | --- |
| NFR-001 | Mandatory TLS verification with no bypass. | P06-002, P11-002 | P06-005, P11-005 |
| NFR-002 | Injection resistance across headers, addresses, filenames, and envelope fields. | P04-002, P07-001, P11-003 | P07-005, P11-005 |
| NFR-003 | No secret, body, attachment, token, or absolute path leakage. | P09-003, P11-004 | P11-005, P12-003 |
| NFR-004 | Bounded memory under all caps. | P08-003, P13-003 | P13-005, P11-005 |
| NFR-005 | Predictable performance with an approved budget. | P13-001–P13-005 | P13-005 |
| NFR-006 | Cross-platform packaging with no untested claims. | P14-003, P14-004 | P14-005 |
| NFR-007 | Deterministic tests with no external network. | P03-004, P12-004 | P12-005 |
| NFR-008 | No sensitive logging. | P09-003, P11-004 | P11-005 |
| NFR-009 | Panic safety across the FFI boundary. | P04-001, P05-004, P11-004 | P11-005 |
| NFR-010 | Strict TypeScript with no `any` in public results/errors. | P09-001 | P12-003 |
| NFR-011 | Reproducible, provenance-tracked release artifacts. | P16-002, P17-001 | P16-005, P17-005 |
| NFR-012 | Documentation accuracy. | P15-005 | P17-002 |
| NFR-013 | Minimal, pinned, licensed dependency set. | P03-002, P11-004 | P16-002 |
| NFR-014 | Deterministic error precedence. | P04-001 | P04-005, P12-002 |

## 5. Compatibility targets

- **Rust:** stable toolchain; minimum recorded in `rust-toolchain.toml` at P03-001; must satisfy napi-rs CLI requirements (Rust 1.88+ observed).
- **Node.js build tooling:** current supported CLI range (Node 22.13+ or 24+ recommended by napi-rs docs observed 2026-11-13).
- **Runtime candidates:** Node.js 22, 24, 26. Support is declared only after P01-004 and P14-004.
- **Node-API:** level 8 proposed.
- **Platforms:** macOS arm64/x64, Linux x64/arm64 glibc, Linux x64/arm64 musl, Windows x64 MSVC proposed. Each target must be built and runtime-tested before being claimed.
- **Module systems:** ESM and CommonJS.
- **Package managers:** npm consumer contract; pnpm workspace proposed.

## 6. Open questions

| ID | Question | Owner | Blocks |
| --- | --- | --- | --- |
| OQ-01 | Does lettre expose per-recipient RCPT status through a lower-level async API? | P01-003 | P06-003, P09-002 |
| OQ-02 | Which timeout stages are directly observable? | P01-003 | P02-003, P10-002 |
| OQ-03 | Which rustls feature set composes with lettre 0.11.23? | P01-002 | P03-002, P06-002 |
| OQ-04 | Does the napi-managed Tokio runtime satisfy lettre on Node 22/24/26? | P01-004 | P05-003 |
| OQ-05 | Which Node-API level do the selected napi features require? | P01-004 | P05-001 |
| OQ-06 | Which targets can be built and runtime-tested in CI? | P14-003 | P14-004, P15-001 |
| OQ-07 | What performance budget is defensible? | P13-001 | P13-005 |

## 7. Traceability

- Feature classification and task mapping: [feature-matrix.md](../planning/feature-matrix.md).
- Requirement-to-test traceability: produced in P12-001.
- Constraint-to-ADR mapping: [ADR index](../architecture/decisions/README.md) and [master roadmap](../planning/master-roadmap.md).
