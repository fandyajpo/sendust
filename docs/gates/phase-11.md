# QG-11 — Security Phase Record

**Decision:** PENDING — awaiting project-owner approval  
**Phase:** P11 — Security  
**Gate:** PHASE-11  
**Prepared:** 2026-11-15  
**Branch:** `main`  
**Evidence:** the local Rust, checker, secret-scan, and binding commands in section 9, and CI on the commit that adds this record.

This record is the review package for PHASE-11. The implementation was written by an
agent, so the approval in section 10 is the required human review step, not a
self-approval. Nothing here approves P12 or any later gate.

## 1. Task evidence

| Task | Status | Evidence |
| --- | --- | --- |
| P11-001 | IN REVIEW | [`docs/security/threat-model-closure.md`](../security/threat-model-closure.md): every abuse case AB-01…AB-14 mapped to its control and test, six residual risks accepted, one medium note carried forward. |
| P11-002 | IN REVIEW | [`docs/security/tls-auth-audit.md`](../security/tls-auth-audit.md): rustls `ring`/`webpki-roots`, hostname verification, TLS 1.2 floor, no verification switch; certificate matrix; PLAIN/LOGIN only, TLS required, forced mechanism must be advertised, `535` redacted and terminal. |
| P11-003 | IN REVIEW | [`docs/security/injection-attachment-audit.md`](../security/injection-attachment-audit.md): CRLF/bare-LF/NUL/reserved-header/IDNA fixtures, filename and path rules, MIME folding and RFC 2231, symlink/FIFO/device/directory/growth matrix, filesystem default, cap placement. One low defect found and fixed (facade copied before checking). |
| P11-004 | IN REVIEW | [`docs/security/ownership-redaction-audit.md`](../security/ownership-redaction-audit.md): copy/detach fixtures, redaction matrix, concurrency/deadline/close/descriptor bounds. The P11-001 panic note is closed with `#[napi(catch_unwind)]`; one low deferral (automated forced-GC test) recorded for P12. |
| P11-005 | IN REVIEW | This record: the acceptance checklist, closure summary, dependency and licence review, secret-scan results, executed evidence, and the pending approval. |

## 2. Acceptance criteria

| Phase-11 criterion | Result | Evidence |
| --- | --- | --- |
| Every Phase 00 abuse case has implementation and test evidence | PASS | Section 3; AB-01…AB-13 closed, AB-14 deferred to P14/P16 by design |
| TLS verification cannot be disabled | PASS | P11-002; `check-architecture` fails on every bypass marker and forbids `native-tls`/`openssl`/`rustls-native-certs` |
| Certificate failures close safely before DATA | PASS | Five bad-certificate fixtures, untrusted root, malformed anchor; every case asserts no `MAIL` |
| Injection, path traversal, special-file, and URL paths are impossible or rejected | PASS | P11-003 fixtures; filename/path normalization; no-follow open; `check-architecture` rejects any URL scheme in source |
| Secrets, bodies, attachments, and sensitive paths are redacted | PASS | P11-004; credential and path assertions render `{error:?} {error}`; no logging crate; binding payload escapes its own framing |
| Concurrency, caps, deadlines, and close are bounded | PASS | One active send with no queue, exact deadline boundaries, bounded close, hard caps before the network and before the facade copy |
| No high-severity finding remains | PASS | Section 8 |

## 3. Abuse-case closure

| ID | State | Note |
| --- | --- | --- |
| AB-01 | CLOSED | Implicit TLS or required STARTTLS; no opportunistic fallback |
| AB-02 | CLOSED | Every certificate failure fails closed before `MAIL` |
| AB-03 | CLOSED | Injection fixtures across address, header, subject, filename, command |
| AB-04 | CLOSED | Caps enforced before the network, before the facade copy, and before any Rust allocation of caller bytes |
| AB-05 | CLOSED | Symlink, FIFO, device, and directory refused before any read |
| AB-06 | CLOSED | Open-then-fstat with a one-byte growth probe |
| AB-07 | CLOSED | Synchronous snapshot in the facade and an owned copy in the binding |
| AB-08 | CLOSED | Connect, operation, and total deadlines with exact boundaries |
| AB-09 | CLOSED | Post-DATA ambiguity is `unknown`, never `accepted`, never retried |
| AB-10 | CLOSED | One active send, `BUSY` before I/O, single close transition |
| AB-11 | CLOSED | Panic sources absent; sync boundary now guarded by `catch_unwind`, async settled by the runtime. The P11-001 medium note is closed |
| AB-12 | CLOSED | Whitelisted error details only; no credential, body, attachment byte, or absolute path |
| AB-13 | CLOSED | Loopback-only harness enforced by the architecture check; secret scan in CI |
| AB-14 | DEFERRED | Platform artifact integrity is owned by P14-003/P16-002, not an MVP code gate |

## 4. TLS and authentication evidence

- Trust store: bundled WebPKI roots plus additive caller anchors (Rust API only); no path can empty or replace the store.
- Certificate matrix: self-signed, expired, wrong host, incomplete chain, untrusted root, malformed anchor, malformed server certificate — all fail before `MAIL`.
- Protocol floor: TLS 1.2, `ring` provider, no client certificate, no version or cipher override exposed.
- Authentication: PLAIN and LOGIN only, never before TLS, forced mechanism must be advertised, `535` is terminal with the SMTP code and no credentials in Display or Debug.

## 5. Injection, MIME, and attachment evidence

- CRLF, bare LF, bare CR, vertical tab, and NUL are `VALIDATION_HEADER_INJECTION` in values, subjects, addresses, filenames, and paths.
- Reserved header names, duplicate custom headers, IDNA conversion, ASCII-only local parts, and overlong parts are rejected deterministically.
- Filenames are display names only: path components stripped, control characters rejected, duplicates refused, Unicode encoded per RFC 2231 with escaped quotes.
- Attachment reads reject symlinks, FIFOs, devices, directories, missing files, and files that grow past the cap; the descriptor is re-inspected after open; the path never appears in an error.
- Filesystem access is off by default in the core, the binding, and the facade.

## 6. Ownership, redaction, and bounds evidence

- Buffer attachments are copied before `send()` returns and survive caller mutation or detachment.
- Error Display and Debug carry only the code, category, and whitelisted details; the credential and path assertions render both forms.
- One active send, no queue, no second connection; exact deadline boundaries; bounded and idempotent close; stable descriptor counts across success, failure, TLS failure, and mixed stress.
- The sync Node-API boundary is panic-guarded; a panicking async task surfaces as `INTERNAL`.

## 7. Dependency and licence review

Direct runtime dependencies and their declared licences:

| Crate | Version | Licence |
| --- | --- | --- |
| `base64` | 0.23.1 | MIT OR Apache-2.0 |
| `idna` | 1.1.0 | MIT OR Apache-2.0 |
| `libc` | 0.2.191 | MIT OR Apache-2.0 |
| `rustls` | 0.23.46 | Apache-2.0 OR ISC OR MIT |
| `rustls-pki-types` | 1.16.0 | MIT OR Apache-2.0 |
| `tokio` | 1.54.1 | MIT |
| `tokio-rustls` | 0.26.6 | MIT OR Apache-2.0 |
| `webpki-roots` | 1.0.12 | CDLA-Permissive-2.0 |
| `napi` / `napi-derive` | 3.16.2 / 3.7.0 | MIT |
| `mail-parser` (test only) | 0.11.9 | Apache-2.0 OR MIT |

The locked graph contains 44 packages declaring Apache-2.0, 27 MIT, 19 Unicode-3.0, 4 ISC, 1 BSD-3-Clause, 1 CDLA-Permissive-2.0, and 2 with no field — the two local `sendust` crates. Every external licence is permissive; no copyleft dependency is present. TypeScript tooling is Apache-2.0 (TypeScript), MIT OR Apache-2.0 (Biome), and MIT (Vitest).

Open items for the project owner, recorded rather than silently accepted:

- The repository declares no `license` field in either manifest and contains no `LICENSE` file. This blocks nothing at MVP (nothing is published) but must be decided before P13/P14 packaging.
- CI runs the pinned-toolchain build, the checkers, and the secret scan, but no advisory (`cargo audit`/`npm audit`) stage. The dependency set is exact-pinned and reviewed here; an automated advisory stage remains an owner decision.

## 8. Findings

| Severity | Finding | State |
| --- | --- | --- |
| Low | Facade copied buffer attachments before checking their size | FIXED in P11-003 |
| Medium | No explicit panic guard on synchronous binding entry points | FIXED in P11-004 |
| Low | No automated forced-GC test | DEFERRED to P12; nothing for the collector to trigger |
| Note | A buffer attachment is copied twice | ACCEPTED; both copies bounded by the cap |
| Note | No declared licence and no advisory stage in CI | OWNER DECISION before packaging |

No high-severity finding is open.

## 9. Executed evidence

Run on macOS with the pinned toolchain on 2026-11-15:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
node scripts/check-architecture.mjs
node scripts/check-docs.mjs
node scripts/check-secrets.mjs --selftest
node scripts/check-secrets.mjs
npm run build:native
npm --workspace ./ts run typecheck
npm --workspace ./ts run lint
npm --workspace ./ts run format:check
npm --workspace ./ts run test
```

Result: 238 Rust tests pass (177 core unit, 26 SMTP fault, 9 harness, 6 client, 6 session, 6 MIME body, 4 MIME conformance, 4 leak probes) and 44 facade tests pass; every checker reports PASS, and the secret scan finds no credential shape outside the five TLS fixture leaf keys it documents.

## 10. Decision

**PENDING.** This record is complete and the evidence above is green, but PHASE-11 is
not approved until the project owner reviews it and answers with an explicit
approval. The phase stays IN PROGRESS until then, and P12 may not begin.

The review should confirm three points:

1. The two defects found during the audits (pre-copy attachment check, panic
   guard) are fixed and covered, and no high-severity finding remains open.
2. The residual risks in section 11 are acceptable for the MVP.
3. The two owner decisions in section 7 (licence declaration, advisory stage)
   may be deferred to packaging.

## 11. Residual risks (proposed for acceptance)

| ID | Risk | Decision |
| --- | --- | --- |
| R-01 | `webpki-roots` does not trust enterprise roots installed only in the OS | Accept; additive anchor path exists on the Rust API |
| R-02 | A close that interrupts DATA reports `unknown` conservatively | Accept; errs toward ambiguity, never a false verdict |
| R-03 | No deterministic DNS-failure fixture | Accept; the connect deadline and reset paths are covered |
| R-04 | No JS-level fake-TLS relay example | Defer to P12 QA |
| R-05 | No automated forced-GC test | Defer to P12; no finalizer exists to run |
| R-06 | Big-endian, musl, and Windows-on-ARM targets untested | Owned by P14-003; no support claim is made |
| R-07 | No declared licence, no advisory stage in CI | Owner decision before P13/P14 packaging |

## 12. Deferred and explicit non-goals

- **Deferred:** platform artifact integrity and provenance (AB-14 → P14-003/P16-002), JS-level relay fixture (P12), forced-GC harness (P12).
- **Non-goals, signed for this gate:** security certification, penetration-test report, SMTPUTF8, connection pooling, automatic retry, internal queue or scheduler, delivery guarantee, and any post-MVP hardening beyond the residual list above.
