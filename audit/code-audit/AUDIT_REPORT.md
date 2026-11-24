# Sendust — Comprehensive Source Code Audit

**Subject:** `@sendust/ts` 1.2.1 — Rust workspace (`sendust-core`, `sendust-napi`) plus the
runtime-neutral TypeScript facade.
**Date:** 2026-11-25.
**Auditor role:** principal Rust engineer, TypeScript library architect, SMTP protocol
specialist, application security auditor, software quality engineer.
**Nature:** audit only. No source, test, manifest, lockfile or workflow was modified.
**Deliverables:** this report and the five companion reviews in this directory.

---

## 0. Method and its limits

### 0.1 What was done

Every production source file was read in full, plus the release workflow, the test
harnesses and the architecture records needed to judge intent against implementation:

| Surface | Files read | Lines |
| --- | --- | --- |
| `crates/sendust-core/src` | 25 of 25 | 8,452 |
| `crates/sendust-napi/src` + `build.rs` | 2 of 2 | 592 |
| `ts/src` | 10 of 10 | 2,059 |
| `.github/workflows/ci.yml` | 1 | 614 |
| Selected tests, fixtures, ADRs, manifests | 12 | ~2,600 |

The remaining test files (≈5,000 lines of Rust integration tests, TypeScript suite) were
located and sampled but not read line by line; where this report relies on them it says so
and never asserts what a test proves beyond what its source shows.

### 0.2 Static only — an explicit limitation

**No test suite, benchmark, compiler or package command was executed for this audit.** The
auditor's environment has no build toolchain, and running the suite would create build
artifacts, which the brief forbids. Consequently:

* Every test result quoted in this report is **the committed test source**, not an observed
  run. Where the project's own documents claim a PASS, this report treats it as a claim
  about a prior run, not as evidence available now.
* No throughput, latency, memory or RSS figure is stated as measured. The performance
  section reasons about allocation counts and algorithmic shape only.
* No dependency was audited against a live advisory database. The supply-chain section
  reasons from pinned versions and the manifest, and says where a real audit is required.

### 0.3 Rules held

No code was fixed. Findings describe the code as it is. Confirmed defects (traceable from
call site to observable behaviour) are separated from risks (a plausible failure the code
permits but no path demonstrates). Each finding carries an ID, severity, category, file and
line, quoted evidence, impact, reproduction conditions and confidence.

---

## 1. Executive summary

Sendust is a serious piece of engineering. The Rust core is disciplined, the error taxonomy
is a genuine contract rather than an afterthought, the TLS posture is correct, and the
tests are adversarial — the fake SMTP server models faults, the DKIM verifier is a second
implementation, the descriptor probes serialise a runtime to avoid false positives. This is
better than the median 1.x native Node addon.

It is not finished. Two defects make headline features unreliable in the field, and a
cluster of medium findings shows a pattern: **contracts were designed twice and only one
copy was wired up.**

### 1.1 The two defects that matter

**H-01 — `AUTH` is capped at 512 bytes while its own validator allows 12,288.**
`Command::to_bytes` validates an AUTH initial response against 12,288 characters
(`proto/command.rs:74`), then rejects the assembled line above `MAX_COMMAND_BYTES = 512`
(`proto/command.rs:13,103`). XOAUTH2's initial response is
`base64("user=<mailbox>\x01auth=Bearer <token>\x01\x01")`, so any bearer token longer than
roughly 330 bytes produces an over-long line. Microsoft 365 and Graph access tokens are
JWTs of typically 800–2,000+ characters, so **XOAUTH2 — the headline feature of 1.1.0 —
cannot authenticate against Microsoft with a normal token**, and it fails with
`VALIDATION_HEADER_INJECTION`, a code that describes an injection attempt. The test suite
does not catch this because its fixtures use short tokens.

**H-02 — a panicking delivery bricks the transport permanently, reporting `BUSY`.**
`Transport::send` takes the native transport out of its slot and moves it into a spawned
task (`sendust-napi/src/lib.rs:321-338, 357-370`). If that task ends in `Err` — a panic is
the only realistic cause — the transport was dropped inside the task and the slot is left
`None` (`lib.rs:405`). Every later `send` finds `slot.take() == None` and returns
`TRANSPORT_BUSY` (`lib.rs:330-337`), which is classified as non-retryable and non-fatal.
The transport never recovers; only constructing a new one does. The reported cause is also
wrong: the real failure is `INTERNAL`.

### 1.2 The pattern behind the medium findings

| Designed contract | Where it is declared | Who actually uses it |
| --- | --- | --- |
| Lifecycle state machine | `core/src/lifecycle/mod.rs` (273 lines, condvar, drain) | Nobody. The binding re-implements it with an `AtomicU8` phase and a tokio mutex. |
| Validation precedence | `core/src/validation/mod.rs` (`validate_in_order`) | Nobody. `OutgoingMessage::validate` runs the stages inline. |
| Error `details.recipient`, `details.smtpEnhancedCode` | `core/src/errors/mod.rs`, `docs/architecture/error-model.md`, `ts/test/errors.test.ts` | Nothing ever sets them; `annotate` only forwards values that never exist. |
| `PROTOCOL_REJECTED` | Same, plus two guides | No producer anywhere in the tree. |
| Relay `SIZE` capability | Parsed in `proto/capabilities.rs:38-40` | Never read; nothing prevents sending 30 MiB to a relay advertising 10 MiB. |

Each is individually a maintainability defect. Together they say something sharper: the
project documents an architecture it then routes around, so the documentation cannot be
trusted as a description of behaviour without reading the code. That is the single biggest
risk to long-term development here — larger than any individual bug.

### 1.3 What is genuinely good

* **TLS cannot be weakened.** No plaintext mode, no `dangerous_*` escape hatch, verification
  unconditional, TLS 1.2 floor, `ServerName` hostname checking, and the STARTTLS plaintext
  injection defence (`if !self.buffer.is_empty() { return Err(...) }`,
  `client/connection.rs:213-215`) — the CVE-2011-0411 class is closed.
* **CRLF injection is blocked at five independent layers**, each of which re-validates at
  its own boundary rather than trusting the caller (`command.rs:129-145`,
  `message/mod.rs:50-70`, `address/mod.rs:112-117`, `attachments/mod.rs:146-158`,
  `attachments/mod.rs:188-198`).
* **Errors cannot leak secrets.** Construction is a code plus a whitelisted detail set; the
  redaction test iterates all 35 codes (`errors/mod.rs:376-396`).
* **The outcome model is honest.** `accepted` requires a positive final `DATA` reply,
  post-`DATA` uncertainty is `unknown` and never retried, and a partially accepted envelope
  keeps per-recipient truth (`transports/smtp/mod.rs:455-593`).
* **The file reader is defensive** — `O_NOFOLLOW | O_NONBLOCK`, then `fstat` on the opened
  descriptor, bounded read with a probe byte (`attachments/mod.rs:276-320`).

### 1.4 Verdict

| Dimension | Assessment |
| --- | --- |
| Overall code quality | **Good.** Consistent style, purposeful comments, few needless allocations. |
| Architecture maturity | **Medium.** Clean dependency direction, undermined by duplicated and dead abstractions. |
| Security posture | **Strong** for a 1.x library; no exploitable defect found. Residual model risks documented. |
| SMTP maturity | **Medium-good.** Correct where implemented; narrow coverage (no SIZE, no 8BITMIME negotiation, no SMTPUTF8, no HELO fallback). |
| Native binding reliability | **Medium.** Sound ownership model; the slot-lifecycle defect in H-02 is a real reliability hole. |
| TypeScript developer experience | **Good.** Frozen narrow surface, `strict` + `exactOptionalPropertyTypes`, unknown-key rejection, typed errors. |
| Maintainability | **Medium.** Excellent module boundaries in Rust; dead modules and doc-vs-code drift undermine them. |
| **Production readiness** | **Ready for password-authenticated SMTP on Node/Bun/Deno. Not ready for XOAUTH2 against large-token providers (H-01) until fixed.** |

Recommendation: **fix H-01 and H-02 before the next release; treat §1.2 as a debt-reduction
programme rather than a backlog item.** Details in `IMPROVEMENT_ROADMAP.md`.

---

## 2. Repository and architecture overview

### 2.1 Shape

```
sendust/
├── Cargo.toml                  workspace: 2 crates, edition 2024, rust-version 1.88
├── crates/sendust-core/        runtime-independent domain + engine (25 files, 8,452 lines)
│   └── src/
│       ├── address/            mailbox parsing, IDNA, injection rejection
│       ├── attachments/        policy, descriptor validation, bounded file reader
│       ├── config/             Limits, Timeouts, SendDeadlines
│       ├── delivery/           send use case: validate → load → serialize → sign → port
│       ├── dkim/               RFC 6376 signer (relaxed/relaxed, rsa-sha256)
│       ├── errors/             append-only code taxonomy + whitelisted details
│       ├── lifecycle/          state machine — NOT used by the shipped path (M-05)
│       ├── message/            OutgoingMessage, HeaderName/HeaderValue newtypes
│       ├── mime/               header rendering/folding, MIME tree, wire accounting
│       ├── ports/              the one DeliveryPort seam
│       ├── results/            SendOutcome, per-recipient status, DeliveryProgress
│       ├── transports/smtp/    the only adapter: session, connection, proto codec
│       └── validation/         stage ordering — NOT used by the shipped path (M-06)
├── crates/sendust-napi/        cdylib: JS conversion, async task, lifecycle bridging
├── ts/                         @sendust/ts: facade, engine, retry policy, loader
└── tests/                      runtime harnesses, TLS fixtures, docs sample, benchmarks
```

### 2.2 How one message travels

Traced from source, not inferred:

1. **`Transport.send(message)`** — `ts/src/transport.ts:550`. Refuses if the engine is not
   `open`; converts the message to a plain native record, copying every buffer
   synchronously (`toNativeMessage` → `new Uint8Array(attachment.content)`, line 396).
2. **`DeliveryEngine.submit`** — `ts/src/engine.ts:181`. Enforces `maxQueue` and
   `maxQueueBytes`, stamps `deadline = now + totalMs`, returns a promise that settles with
   *this* message's outcome.
3. **Scheduling** — `engine.ts:265-336`. `#pump` grows concurrency to meet demand,
   applies the token-bucket rate limit, acquires a native transport from the pool or creates
   one, then calls `transport.send(nativeMessage)`.
4. **Native boundary** — `sendust-napi/src/lib.rs:308`. Checks the phase, reserves the
   single-send slot, **synchronously** rebuilds the message (`build_message`, line 464:
   `Mailbox::parse`, `Header::new`), spawns a tokio task, records the abort handle.
5. **Orchestration** — `Delivery::send`, `core/src/delivery/mod.rs:50`. Validates, reads
   file attachments, serializes at one `SystemTime`, DKIM-signs, re-checks the wire cap,
   then hands `DeliveryRequest { message, wire }` to the port.
6. **Composition** — `mime::compose`, `core/src/mime/mod.rs:56`. Fixed header order, RFC
   2047 encoding and folding, base64 bodies, `multipart/alternative` and `multipart/mixed`,
   final cap check.
7. **Transport** — `SmtpTransport::run`, `transports/smtp/mod.rs:352`. Starts a deadline
   clock, builds the envelope, acquires or reuses a session, runs one transaction.
8. **Session** — `Session::open` → `Connection::connect` (TCP, implicit TLS or plaintext
   greeting, EHLO, capabilities, optional STARTTLS + re-EHLO, optional AUTH).
9. **Transaction** — `MAIL FROM`, one `RCPT TO` per recipient observing each reply, `DATA`,
   dot-stuffed body with terminator, final reply, outcome mapping.
10. **Return** — `SendOutcome` → `SendResultJs` → `toSendResult` → caller's promise.

### 2.3 Dependency direction

`ts → (napi) → sendust-core → { rustls/tokio/base64/idna/rsa/sha2 }`. The core never
references `napi`, Node, or a JS runtime; `scripts/check-architecture.mjs` enforces the
module boundaries in CI. The direction is correct and consistently held — the one breach is
behavioural rather than structural: the binding implements lifecycle policy that ADR-0016
assigns to the core domain (M-05).

---

## 3. Findings index

Severity is assigned by consequence, not by how easy the defect is to trigger. Full evidence
is in the named review.

### HIGH

| ID | Title | Area | Full record |
| --- | --- | --- | --- |
| H-01 | `AUTH` line capped at 512 bytes; XOAUTH2 fails for large bearer tokens | SMTP protocol | SECURITY §4 |
| H-02 | A failed async delivery leaves the transport permanently `BUSY` | Native binding | CODE_QUALITY §3.2 |

### MEDIUM

| ID | Title | Area | Full record |
| --- | --- | --- | --- |
| M-01 | Cancelling a send during composition reports the *previous* message's recipients as `unknown` | Reliability | CODE_QUALITY §6 |
| M-02 | A pool transport is released twice if the native result fails mapping | TypeScript engine | CODE_QUALITY §2.3 |
| M-03 | `RSET`'s own reply is never checked before a session is reused | SMTP protocol | CODE_QUALITY §6 |
| M-04 | The relay's `SIZE` capability is parsed and ignored | SMTP protocol | ARCHITECTURE §5 |
| M-05 | The core lifecycle module is dead; lifecycle is enforced twice | Architecture | ARCHITECTURE §5 |
| M-06 | `validate_in_order` is unused; precedence is inline | Architecture | ARCHITECTURE §5 |
| M-07 | `host` accepts embedded whitespace contrary to the documented contract | Validation | SECURITY §4 |
| M-08 | Attachment paths have no root constraint once file access is enabled | Security | SECURITY §4 |
| M-09 | One `AUTH` failure stops the transport permanently, undocumented | Reliability | SECURITY §4 |
| M-10 | `details.recipient` and `details.smtpEnhancedCode` can never be produced | API contract | ARCHITECTURE §5 |
| M-11 | `PROTOCOL_REJECTED` is documented and unproducible | API contract | ARCHITECTURE §5 |
| M-12 | `tsc` typechecks 2 of 7 test files | TypeScript | CODE_QUALITY §2.3 |
| M-13 | No streaming: peak memory is a multiple of the message | Performance | ARCHITECTURE §7 |

### LOW (15) and INFORMATIONAL (10)

Enumerated in `CODE_QUALITY_REVIEW.md` §7 as L-01…L-15 and I-01…I-12, with the
security-relevant observations discussed in `SECURITY_REVIEW.md` §7 and the compatibility gaps
in `COMPATIBILITY_REVIEW.md` §7. None affects correctness of the accepted path.

### Counts

2 HIGH · 13 MEDIUM · 15 LOW · 10 INFORMATIONAL · **0 CRITICAL**.

No defect found in this audit permits header injection, credential disclosure, TLS
downgrade, arbitrary file read without explicit opt-in, or delivery to an unintended
recipient.

---

## 4. Security summary

Full analysis in `SECURITY_REVIEW.md`. Headline positions:

* **No exploitable vulnerability was identified.** The injection surface is closed by
  construction and by re-validation at each boundary.
* The trust model is explicit and correct: mandatory TLS, public WebPKI roots plus optional
  additive anchors, no pinning, no DANE, no MTA-STS. Residual risk is CA-level, which is the
  documented position (ADR-0003, ADR-0018).
* **Two model risks deserve documentation, not code change:** attachment paths are trusted
  once `allowFileAccess` is enabled (M-08), and the DKIM key plus any bearer token exist as
  immutable JavaScript strings whose lifetime the library cannot control (I-08).
* Supply chain is strong: every Rust dependency is pinned exactly, `Cargo.lock` is committed
  and `--locked` is used in CI, npm publishes with OIDC trusted publishing and a SLSA
  provenance attestation, and no install-time script runs.

---

## 5. Reliability summary

The delivery correctness model is the strongest part of the design. Confirmed:

* `accepted` only after a positive final `DATA` reply (`smtp/mod.rs:550-560`).
* Post-`DATA` failure of any kind — including a write timeout — maps to `unknown`, not an
  error and not a retry (`smtp/mod.rs:580-591`). This is conservative and correct.
* Only transient, provably pre-acceptance failures are retried
  (`ts/src/policy.ts:56-61`); `unknown` is terminal (`policy.ts:97-101`).
* Per-recipient truth survives partial acceptance (`smtp/mod.rs:484-524`).

The reliability gaps are in the wrapper, not the protocol: H-02 (permanent busy), M-01
(misattributed unknown), M-02 (double release), M-09 (one auth failure stops everything).

---

## 6. Compatibility summary

Details and per-cell evidence in `COMPATIBILITY_REVIEW.md`.

| Claim | Status |
| --- | --- |
| Node.js 22.23.5 / 24 / 26 on linux-x64 | **Supported** — built, loaded and exercised per target in CI |
| Node.js on linux-arm64, macos-arm64, macos-x64, windows-x64 | **Supported** — native runners, no cross-build |
| Alpine/musl linux-x64, linux-arm64 | **Supported** — built inside the musl image itself |
| Bun 1.4.4, Deno 2.9.8 on those targets | **Supported, indirectly** — smoke + facade + behaviour harness per target |
| Node.js on win32-arm64 | **Unsupported** — no package; fails closed with `CONFIG_NATIVE_UNAVAILABLE` |
| Node.js < 22.23.5 | **Unsupported by declaration** (`engines`) |
| Bun/Deno on Windows-arm64, or on musl without the package | **Unknown / unsupported**, fails closed |

The compatibility discipline is unusually good: distribution is per-platform optional
packages, and a missing build fails closed rather than falling back to a compiled-at-install
path.

---

## 7. Production readiness

**Usable now** for: password (PLAIN/LOGIN) authentication against mainstream relays, on
Node 22+, Bun or Deno, on any of the seven published targets, with or without DKIM.

**Blocked** for: XOAUTH2 against providers issuing long bearer tokens (H-01), and for any
workload where one internal panic must not take the transport out of service (H-02).

**Before calling it stable for a fleet:** fix H-01, H-02; add the `SIZE` guard (M-04);
correct the four dead/duplicated contracts (M-05, M-06, M-10, M-11) so the documentation
describes the code again.

---

## 8. Confidence and open questions

| Claim | Confidence | Basis |
| --- | --- | --- |
| H-01 mechanism | **High** | Two constants and one comparison in one file; no ambiguity |
| H-01 field impact | **Medium-high** | Depends on provider token length; typical M365/Graph JWTs exceed the limit, but no real token was tested |
| H-02 mechanism | **High** | Slot emptied before the task runs, never refilled on `Err` |
| H-02 trigger frequency | **Low measured** | No panic path was found in normal operation; the defect is the consequence, not the cause |
| M-01 window | **High** | `reset_progress` runs inside `run`, after validation and composition |
| No exploitable injection | **High** | Every producer re-validates; traced end to end |
| Performance statements | **Low** | Static reasoning only; no benchmark was run |

Questions this audit cannot answer without execution: the real XOAUTH2 token lengths in
production; whether any dependency carries a current advisory; measured behaviour under the
concurrency ceiling; and whether the musl detection heuristic misfires on any real host (I-11).
