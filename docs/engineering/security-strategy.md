# Security Strategy

**Applies to:** all phases, with enforcement concentrated in P06, P08, P09, P11, P14, P16.  
**Requirements:** C-07, C-11, C-12, C-19, C-20, C-21, C-29; NFR-001, NFR-002, NFR-003, NFR-009.  
**Related:** [error model](../architecture/error-model.md), [ADR-0003](../architecture/decisions/0003-tls-rustls-ring-webpki.md), [ADR-0008](../architecture/decisions/0008-owned-memory.md), [risk registry](../planning/risk-registry.md).

## 1. Assets and trust boundaries

| Asset | Location | Trust |
| --- | --- | --- |
| SMTP credentials | Consumer config, JS heap, owned Rust memory | Secret; never logged or serialized |
| Message body | JS heap, owned Rust memory, wire bytes | Potentially sensitive; never logged |
| Attachment bytes and paths | JS heap, filesystem, owned Rust memory | Sensitive; path redacted |
| Recipient addresses | JS heap, headers, envelope | Personal data; not a secret but not logged in bulk |
| Process availability | Node process | Must not be terminable by malformed input |
| Relay trust | Network | Untrusted until TLS verification succeeds |

Trust transitions:

1. Consumer → wrapper: untrusted input shape.
2. Wrapper → binding: untrusted values crossing FFI.
3. Binding → core: owned Rust values; still untrusted content.
4. Core → lettre: validated envelope and serialized message.
5. Core → filesystem: only with explicit opt-in and regular-file checks.
6. Core ↔ relay: untrusted network; TLS mandatory.
7. Core → error/result → wrapper → consumer: redacted output.

## 2. Threat model summary

| Threat | Vector | Control | Verification |
| --- | --- | --- | --- |
| TLS downgrade / MITM | Plaintext SMTP, disabled verification, permissive roots | No plaintext mode; verification mandatory; `webpki-roots`; TLS 1.2 minimum | P06-005, P11-002 |
| Credential theft | Error messages, logs, diagnostics | Redaction rules; no password in `details` | P09-003, P11-004 |
| Header injection | CRLF/NUL in display names, subjects, filenames, custom headers | Reject control characters; reserved-header protection | P07-005, P11-003 |
| Envelope injection | Malformed addresses, extra recipients | Strict address parsing; recipient cap; no raw header pass-through | P04-005, P11-003 |
| SSRF / local file read | URL or path attachments | No URL type; opt-in FS; regular files only; no globs | P08-005, P11-003 |
| Memory corruption | Borrowed JS buffers, unsafe FFI | Copy before async; no borrowed memory; unsafe audit | P05-005, P11-004 |
| Process crash | Rust panic across FFI | `Result`; `catch_unwind`; no abort strategy | P11-005 |
| DoS | Oversized recipients/attachments/wire, slow relay | Hard caps; finite timeouts; bounded close | P10-004, P13-003 |
| Supply chain | Compromised or vulnerable dependency | Pin, license review, SBOM, advisory monitoring | P03-002, P16-002 |
| Secret leakage in tests | Real credentials or relays | Fake relay only; generated dummy credentials | P12-004 |

## 3. TLS policy

- Modes: implicit TLS and STARTTLS only. Plaintext is not implemented.
- rustls 0.23 with `ring` provider and `webpki-roots` trust anchors, pending P01-002 validation.
- Certificate chain and hostname verification are always on.
- TLS 1.2 minimum; TLS 1.3 preferred.
- No public option, environment variable, config file, or undocumented flag can disable verification.
- SNI uses the configured relay host.
- Certificate failures close the connection before AUTH or MAIL.
- Enterprise private CAs are not supported in MVP; documented limitation.

## 4. Authentication policy

- PLAIN and LOGIN only; unsupported mechanisms rejected at configuration time.
- Credentials are owned Rust strings, never copied into error messages, logs, or result details.
- AUTH failure is terminal for the send; no retry with alternate credentials.
- Credentials are not persisted by Sendust.

## 5. Input validation

- Address local parts: ASCII only; non-ASCII rejected with `VALIDATION_NON_ASCII_LOCAL_PART`.
- Domains: IDNA conversion; invalid labels rejected.
- Display names, subjects, bodies: Unicode allowed; control characters rejected.
- Headers: valid field-name tokens; reserved names cannot be overridden; values reject CRLF, bare LF, NUL, and other control characters.
- Filenames: control characters rejected; RFC 2231/2047 encoding.
- EHLO identity: hostname token; no CRLF.
- Options: unknown keys rejected; numeric ranges checked; no silent truncation.
- Limits: cannot be raised above hard caps.

## 6. Attachment security

- Filesystem access is disabled unless explicitly enabled.
- Only regular files are read; open with no-follow semantics where supported; `fstat` the descriptor.
- Symlinks, directories, FIFOs, devices, and sockets are rejected.
- Read at most cap + 1 byte to detect growth and overflow.
- Absolute paths are redacted from public errors.
- No URL, stream, glob, or directory attachment form.
- Buffer attachments are copied before async work.

## 7. Memory and FFI safety

- No borrowed JS memory crosses an `await`.
- Every `Buffer`, `ArrayBuffer`, typed array, and string is copied.
- `unsafe` blocks are minimized, justified, and audited.
- Panics are reserved for internal invariants; expected failures use `Result`.
- `catch_unwind` is applied at exported boundaries where the panic strategy permits.
- `panic = "abort"` is not used for the addon.
- No custom Tokio runtime is created.

## 8. Error and log redaction

Never include:

- passwords, tokens, auth challenges,
- full message bodies,
- attachment bytes,
- absolute local paths,
- internal panic payloads.

Allowed:

- host and port,
- SMTP numeric response code and server text if it contains no credentials,
- recipient addresses supplied by the caller,
- timeout category and elapsed budget,
- caller-supplied filenames.

Redaction is verified by serializing errors and scanning for marker values.

## 9. Denial-of-service bounds

- 100 recipients.
- 10 MiB per attachment; 20 MiB aggregate raw; 30 MiB wire.
- Connect 10 s, operation 30 s, total 120 s, close 125 s.
- One active send per transport; no internal queue, worker, scheduler, persistence, or queue adapter. Queue/job orchestration is caller-owned.
- No unbounded buffering or collection growth.
- Close is bounded and cannot hang indefinitely.

## 10. Dependency and supply-chain security

- Pin exact versions; commit the lockfile.
- Review licenses and advisories before adding a dependency.
- Keep the dependency set minimal.
- Generate an SBOM/dependency inventory for each release.
- Publish with provenance.
- Treat native build inputs as part of the trusted computing base.

## 11. Test-environment security

- No real relay, external network, or production credentials in tests.
- Generated test certificates only.
- Secret scanning in CI.
- Network-denial test for the test suite where feasible.
- Test artifacts retained with gate records but free of secrets.

## 12. Security review checkpoints

| Phase | Review |
| --- | --- |
| P01 | TLS feature validation; no verification bypass |
| P03 | Dependency and license policy; secret scanning |
| P05 | Ownership and panic safety |
| P06 | TLS/auth behavior; fault matrix |
| P08 | Filesystem safety and caps |
| P09 | Error redaction and public surface |
| P11 | Full threat-model audit and remediation |
| P14 | Artifact and install integrity |
| P16 | SBOM, provenance, secret scan |
| P17 | Final security sign-off before publication |

## 13. Incident response

- A security defect blocks release; it is never waived.
- If a released version has a security defect: publish a fixed version, deprecate the affected version, document the issue, and avoid replacing binaries under the same version.
- Credential exposure in a published artifact triggers immediate deprecation and consumer notification.
- Post-incident: add a regression test and update this strategy.
