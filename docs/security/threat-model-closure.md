# Threat-Model Closure and Trust-Boundary Audit (P11-001)

**Status:** reviewed — every abuse case owned, residual risks recorded  
**Date:** 2026-11-15  
**Scope:** the six trust boundaries in [threat-model.md](../discovery/threat-model.md), every abuse case AB-01…AB-14, and the controls that now exist in the built system.

This is the closure matrix for PHASE-11. It walks each abuse case across the JS
caller, the Node-API boundary, the Rust core, the filesystem, TLS, and the relay,
and points at the implementation and the test that proves it. Nothing here is a
certification: it is the evidence package P11-002…P11-004 deepened and P11-005
gates in [the PHASE-11 record](../gates/phase-11.md).

## 1. Trust-boundary walk

| Boundary | Control | Evidence |
| --- | --- | --- |
| Caller → TypeScript | Typed surface with exact `keyof` locks; unknown option keys rejected; no `any` | `ts/test/types.test.ts`, `createTransport` key validation, `check-architecture` any rule |
| TypeScript → Node-API | Buffers snapshotted synchronously; options copied into owned values | `copies a buffer attachment before the send returns`, `snapshots a buffer before the caller detaches it` |
| Node-API → core | Owned `OutgoingMessage`; no borrowed JS memory after the first await | Ownership tests, `Attachment::Bytes` construction |
| Core → filesystem | Opt-in only; `symlink_metadata` type gate; `O_NOFOLLOW`/`O_NONBLOCK`; `fstat` recheck; bounded read | P08-002 evidence and `file_reads` suite |
| Core → network | rustls `ring` + `webpki-roots`; required STARTTLS or implicit TLS; finite deadlines; no retry | P11-002, P06/P10 evidence |
| CI → registry | Exact pins, `--locked`, secret scan in CI, licence inventory in the PHASE-11 record | `check-docs`, `check-secrets`, [PHASE-11 record](../gates/phase-11.md) |

## 2. Abuse-case closure

| ID | Abuse case | Control | Evidence | Status |
| --- | --- | --- | --- | --- |
| AB-01 | MITM or STARTTLS stripping | Implicit TLS or required STARTTLS, never opportunistic fallback | `starttls_is_required_and_fails_closed_without_the_extension`, `refuses_starttls_when_not_advertised`, `upgrades_with_starttls_and_rehandshakes` | CLOSED |
| AB-02 | Expired / self-signed / wrong-host / incomplete / malformed certificate | Fail closed in the rustls verifier; no bypass API | `every_bad_certificate_fixture_fails_before_mail`, `an_untrusted_certificate_fails_closed`, `fails_closed_for_each_bad_certificate_fixture`, `rejects_a_malformed_trust_anchor` | CLOSED |
| AB-03 | CRLF/control injection in address, header, filename, EHLO | Header newtypes reject controls; reserved names blocked; filenames stripped and checked | `rejects_control_characters_in_names_as_injection`, `rejects_header_injection_in_values`, `rejects_subject_injection`, `rejects_reserved_custom_headers`, `rejects_header_injection_in_address_and_display_name`, `rejects_invalid_filenames_and_content_types` | CLOSED |
| AB-04 | Oversized recipients, attachment, or wire payload | Hard caps before DATA; deterministic validation order | `recipient_cap_is_inclusive`, `enforces_the_per_file_cap_inclusively`, `enforces_the_aggregate_cap`, `wire_cap_is_inclusive_and_exact`, `a_per_file_violation_precedes_an_aggregate_one` | CLOSED |
| AB-05 | Symlink / FIFO / device / directory path | Filesystem opt-in; type gate; no-follow open; descriptor recheck | `rejects_file_attachments_without_opt_in`, `rejects_a_symlink_to_a_regular_file`, `rejects_a_fifo_without_blocking`, `rejects_a_character_device`, `rejects_a_directory` | CLOSED |
| AB-06 | File changes or grows during read | Open-then-fstat plus one-byte overflow probe | `rejects_a_file_that_outgrows_the_cap`, `enforces_the_per_file_cap_inclusively` | CLOSED |
| AB-07 | Caller mutates or detaches a buffer after `send()` | Synchronous snapshot in the facade and a copy in the binding | `copies a buffer attachment before the send returns`, `snapshots a buffer before the caller detaches it` | CLOSED |
| AB-08 | Relay stalls at any stage | Connect, operation, and total deadlines; bounded close | `connect_boundary_is_exact`, `operation_boundary_is_exact_and_capped_by_the_total`, `total_boundary_is_exact`, `connect_stall_is_a_connect_timeout`, `auth_stall_is_an_operation_timeout`, `rcpt_stall_is_an_operation_timeout`, `data_command_stall_is_an_operation_timeout`, `total_stall_is_a_total_timeout` | CLOSED |
| AB-09 | Connection drops after the DATA terminator | `unknown` outcome, no retry, no DATA-state QUIT | `missing_final_reply_after_data_is_unknown`, `final_reply_stall_after_data_is_unknown`, `a_cancellation_after_data_reports_unknown_for_the_accepted_subset` | CLOSED |
| AB-10 | Concurrent send or send/close race | Atomic phase/slot reservation; BUSY before I/O; close transitions once | `a_second_send_is_busy_and_dials_nothing`, `rejects a second concurrent send with TRANSPORT_BUSY`, `refuses a send that starts during close`, lifecycle suite | CLOSED |
| AB-11 | Panic crosses the Node-API boundary | Production code contains no `unwrap`/`expect`/`panic!` outside tests; failures use `Result`; the sync boundary is panic-guarded and async tasks settle through `JoinError` | `grep` audit of `crates/sendust-core/src` and `crates/sendust-napi/src`; `#[napi(catch_unwind)]` on `new`/`state`; `Err(_) => INTERNAL` in `Transport::send` | CLOSED |
| AB-12 | Credentials, body, or path leak through diagnostics | Structured errors with whitelisted details; no payload tracing | `every_code_renders_without_a_secret_marker`, `redacts credentials, bodies, and paths from every error`, `errors_never_carry_the_path` | CLOSED |
| AB-13 | External relay or real secret in tests | Loopback-only harness enforced by the architecture check; secret scan in CI | `check-architecture` loopback rules, `check-secrets` | CLOSED |
| AB-14 | Platform binary mismatch or compromised artifact | Exact target matrix, checksums, SBOM, provenance, clean installs | Not implemented in code yet | DEFERRED — owned by P14-003/P16-002, not a code gate on the MVP |

## 3. Residual risks (accepted)

| ID | Risk | Decision |
| --- | --- | --- |
| R-01 | `webpki-roots` does not trust private enterprise roots installed only in the OS | Accepted MVP trade-off; documented in the threat model and public API guidance |
| R-02 | A close that interrupts DATA reports `unknown` conservatively — the stage is marked just before the body write, so an abort in that instant still reports `unknown` | Accepted; errs toward ambiguity, never toward a false `accepted` or `rejected` |
| R-03 | DNS failure has no deterministic fixture (needs a live resolver) | Accepted; resolver failures surface through the covered connect deadline and reset paths (P10 record) |
| R-04 | No JS-level fake-TLS relay example | Deferred to P12 QA; transaction faults are covered by the Rust harness |
| R-05 | No automated forced-GC test | Deferred to P12; nothing finalizer-driven exists for the collector to run |
| R-06 | Big-endian, musl, and Windows-on-ARM targets are untested | Owned by P14-003; no support claim is made |

## 4. Findings

No high-severity finding is open. The one medium note carried out of this audit —
no explicit panic guard in the binding — was fixed in P11-004: the constructor and
`state` now carry `#[napi(catch_unwind)]`, and async methods are settled by the
napi-rs runtime. AB-11 is closed with no residual.

## 5. Ownership

Every abuse case above is owned: AB-14 by P14/P16, R-04 and R-05 by P12, and the
rest by the implemented control and its test. No case is unowned. P11-002…P11-004
deepened the TLS/auth, validation/attachment, and ownership/redaction rows, and
P11-005 gates them in the [PHASE-11 record](../gates/phase-11.md).
