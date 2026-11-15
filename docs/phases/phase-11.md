# Phase 11 — Security

**Phase status:** COMPLETED  
**Phase dependency:** Phase 10  
**Companion references:** [security strategy](../engineering/security-strategy.md), [system design](../architecture/system-design.md), [testing strategy](../engineering/testing-strategy.md), [reliability strategy](../engineering/reliability-strategy.md)

## Objective

Verify and harden the MVP against credential leakage, TLS downgrade, header/address injection, unsafe file access, memory-boundary violations, oversized inputs, and denial-of-service through unbounded waits or concurrency.

## Scope

In scope: threat-model execution, TLS verification proof, secret redaction, input validation, attachment sandboxing, dependency/supply-chain review, denial-of-service bounds, and security regression tests. Out of scope: new product features, SMTPUTF8, pooling, retries, external relay hardening, and security certification. Constraints from discovery apply from Phase 00 onward; this phase proves them.

## Tasks

### P11-001 — Execute the threat model and trust-boundary audit

- **Size:** L
- **Status:** COMPLETED
- **Prerequisites:** P10-005, P00-004
- **Description:** Walk each Phase 00 abuse case across JS caller, napi boundary, core, filesystem, TLS, and relay. Confirm mitigations exist for credential leakage, CRLF/header injection, TLS downgrade, symlink/FIFO/device access, buffer mutation/detachment, oversized envelopes, concurrency, and shutdown hangs. Record residual risks and explicit non-goals.
- **Evidence criteria:** Abuse-case matrix maps each case to implementation and test evidence. No case is unowned. Residual risks are accepted explicitly. Any missing mitigation has a blocking remediation task or gate rejection.

### P11-002 — Prove TLS and authentication security posture

- **Size:** M
- **Status:** COMPLETED
- **Prerequisites:** P11-001, P06-002
- **Description:** Audit rustls `ring`/`webpki-roots` configuration and ensure certificate and hostname verification cannot be disabled by public or internal options. Verify authentication failures do not leak credentials and unsupported auth modes are rejected. Confirm there is no plaintext fallback or insecure toggle.
- **Evidence criteria:** Configuration review finds no verification bypass. Self-signed, expired, wrong-host, incomplete-chain, and malformed certificates fail before DATA. Auth `535` tests show redacted errors and no credential in logs or traces. Feature review confirms no prohibited TLS backend.

### P11-003 — Audit validation, MIME, and attachment security

- **Size:** L
- **Status:** COMPLETED
- **Prerequisites:** P11-001, P07-005, P08-005
- **Description:** Verify address parsing, IDNA, header encoding, MIME rendering, filename handling, and attachment reads against injection, path traversal, special files, TOCTOU, and resource exhaustion. Confirm filesystem access is disabled by default and URLs are impossible. Check all caps are enforced before network and before unbounded allocation.
- **Evidence criteria:** Injection fixtures for CRLF, bare LF, NUL, header-like names, and path-like filenames fail safely. Attachment fixtures for symlink, FIFO, device, directory, growing file, and over-limit fail without partial sends. Filesystem-disabled default is proven. No URL code path exists.

### P11-004 — Audit memory ownership, redaction, and denial-of-service bounds

- **Size:** M
- **Status:** COMPLETED
- **Prerequisites:** P11-001, P10-003
- **Description:** Verify all JS buffers are copied before async, no borrowed memory survives, and errors/logs redact secrets, bodies, attachment bytes, and sensitive paths. Confirm one active send, immediate `BUSY`, finite deadlines, bounded close, hard caps, and no unbounded queues or retry loops. Include process-exit and forced-GC scenarios.
- **Evidence criteria:** Mutation/detachment tests pass. Redaction scans find no credentials, body markers, attachment markers, or absolute paths. Concurrency tests show no queue and no second connection. Timeout/close tests show no unbounded wait. GC cannot close or corrupt a transport.

### P11-005 — Security regression gate and remediation sign-off

- **Size:** M
- **Status:** COMPLETED
- **Prerequisites:** P11-001, P11-002, P11-003, P11-004
- **Description:** Run the security regression suite, dependency/license review, secret scan, and threat-model closure. Record accepted residual risks and any post-MVP hardening. Block the phase if any high-severity finding lacks a fix or explicit rejection.
- **Evidence criteria:** Gate report includes abuse-case closure, TLS/auth evidence, injection/attachment evidence, ownership/redaction evidence, dependency and secret scan results, and residual-risk acceptance. No high-severity finding is open. Phase 12 may begin only after approval.

## Technical implementation details

Security constraints are mandatory from Phase 00 and are proven here. The architecture uses two crates with core free of Node linkage, `TS -> napi -> core -> native SMTP engine`. TLS uses rustls with `ring` and `webpki-roots`; verification is never disableable. Authentication failures are terminal and redacted. Envelope local parts are ASCII, domains IDNA, display/body Unicode; CRLF and NUL are rejected. Attachments require explicit filesystem opt-in, read bounded regular files only, reject symlinks/FIFOs/devices/directories, and never fetch URLs. Caps are hard: 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw, 30 MiB wire. JS buffers are copied before async. One active send returns `BUSY` with no queue. Timeouts are finite: connect 10s, operation 30s, total 120s, close 125s. Close is explicit and not GC-dependent. No pooling, no automatic retries, no SMTPUTF8, no delivery guarantee, and no per-recipient independent messages.

## Dependencies

Phase 11 requires Phase 10 and the Phase 00 boundary. P11-001 depends on P10-005 and P00-004; P11-002 adds P06-002; P11-003 adds P07-005 and P08-005; P11-004 adds P10-003; P11-005 depends on all. Phase 12 requires P11-005.

## Risks and edge cases

Security regressions often appear as convenience features: a TLS-skip flag, URL attachments, retry-after-unknown, or a per-recipient loop. Special-file races, IDNA homographs, encoded-word injection, secret leakage in debug formatting, and unbounded memory from encoded attachments are key risks. Logs and panic messages can leak credentials even when API errors are redacted.

## Testing requirements

Run all security fixtures locally: five TLS certificate failures, auth `535` with credential markers, CRLF/bare-LF/NUL/header-like injection, IDNA edge cases, non-ASCII local-part rejection, symlink/FIFO/device/directory/missing/permission-denied/growing files, filesystem-disabled default, per-file/aggregate/wire over-limit, Buffer mutation/detachment, forced GC, concurrent BUSY, close deadline, and process exit. Add secret scans over logs, errors, panic output, and repository fixtures. Add dependency feature inspection proving no insecure TLS toggle.

## Acceptance criteria

- Every Phase 00 abuse case has implementation and test evidence.
- TLS verification cannot be disabled and all certificate failures close safely.
- Injection, path traversal, special-file, and URL paths are impossible or rejected.
- Secrets, bodies, attachments, and sensitive paths are redacted.
- Concurrency, caps, deadlines, and close are bounded; no high-severity finding remains.

## Deliverables

Threat-model closure matrix, TLS/auth audit, validation/MIME/attachment audit, ownership/redaction audit, security regression suite, dependency/secret scan results, and security gate record.

## Completion gate

`PHASE-11` is approved only when P11-005 closes all high-severity findings or explicitly rejects them with rationale, the security regression suite passes, and no convenience feature undermines the constraints. Phase 12 may begin only after this gate.

## Evidence log

Evidence labels follow the [AI implementation contract](../engineering/ai-implementation-contract.md).

| Task | Evidence | State |
| --- | --- | --- |
| P11-001 | [`docs/security/threat-model-closure.md`](../security/threat-model-closure.md): the six trust boundaries walked with their controls, every abuse case AB-01…AB-14 mapped to implementation and test evidence, the four deferred cases assigned to P12/P14/P16, six residual risks accepted explicitly, and one medium note (no explicit panic guard in the binding) carried into P11-004. No case is unowned and no high-severity finding is open. | COMPLETED |
| P11-002 | [`docs/security/tls-auth-audit.md`](../security/tls-auth-audit.md): the rustls configuration, trust store, hostname checks, protocol floor, and the absence of any verification switch; the certificate failure matrix; the authentication matrix (PLAIN/LOGIN only, TLS required, forced mechanism must be advertised, `535` terminal with no `MAIL`). The architecture check now fails on any TLS bypass marker and on `native-tls`, `openssl`, or `rustls-native-certs`. `auth_failure_is_an_error_and_never_mails` renders `{error:?} {error}` and asserts neither the password nor the username appears. | COMPLETED |
| P11-003 | [`docs/security/injection-attachment-audit.md`](../security/injection-attachment-audit.md): CRLF/bare-LF/NUL/reserved-header/IDNA/overlong fixtures mapped to their codes, filename and path normalization, MIME folding and RFC 2231 encoding, the symlink/FIFO/device/directory/growth matrix, the filesystem-disabled default, and cap placement. The audit found one low defect — the facade copied buffer attachments before any size check — fixed by `ts/src/limits.ts`, which rejects over-cap buffers before the copy and is now covered by `limits.test.ts` and two facade tests. `check-architecture` gained a cap-parity check between `Limits::default()` and `DEFAULT_LIMITS`, and a URL-scheme scan over the core, the binding, and the shared TypeScript. `a_symlinked_file_never_reaches_the_port` proves a non-regular file never dials. | COMPLETED |
| P11-004 | [`docs/security/ownership-redaction-audit.md`](../security/ownership-redaction-audit.md): the copy-before-return and detach fixtures, the redaction matrix (errors, credentials, paths, body bytes, binding payload, no logging crate, secret scan), the concurrency/deadline/close/descriptor bounds, and the panic boundary. The P11-001 note is closed: `Transport::new` and `Transport::state` now carry `#[napi(catch_unwind)]`, and async methods are settled by the napi-rs runtime, which rejects the promise instead of unwinding across the FFI. The audit records one low deferral (no automated forced-GC test) for P12. | COMPLETED |
| P11-005 | [`docs/gates/phase-11.md`](../gates/phase-11.md): the PHASE-11 review package — acceptance checklist, AB-01…AB-14 closure (AB-11 now closed, AB-14 deferred by design), TLS/auth and injection/attachment and ownership evidence, a licence inventory of every direct dependency and the locked graph (no copyleft; two unlicensed local crates), the secret-scan result, the executed command set with 238 Rust and 44 facade tests, two owner decisions (licence declaration, advisory stage), and the pending approval. | COMPLETED |

## Status

COMPLETED. P11-001 to P11-005 are COMPLETED and PHASE-11 is APPROVED (project owner, 2026-11-15). Phase 12 is eligible.
