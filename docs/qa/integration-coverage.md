# Integration Coverage Report (P12-002)

**Status:** complete — every suite green, gaps named
**Date:** 2026-11-15
**Method:** the pinned toolchain on macOS; `cargo test --workspace --all-targets --locked`, `npm run build:native`, and `npm --workspace ./ts run test`, each run twice, plus a Node-free run of the core suite. This report maps suites to modules, names the critical branches and their tests, and lists what is *not* covered rather than claiming otherwise.

## 1. Executed suites

| Suite | Location | Tests | Proves |
| --- | --- | --- | --- |
| Core unit | `crates/sendust-core/src/**` | 177 | Each module's own contract, in isolation |
| Harness self-test | `tests/harness_self_test.rs` | 9 | The fake relay scripts, traces, and TLS fixtures behave as documented |
| SMTP fault matrix | `tests/smtp_faults.rs` | 26 | Stalls, resets, malformed replies, auth failures, TLS failures, ambiguity |
| SMTP session | `tests/smtp_session.rs` | 6 | Full transactions over loopback TLS, STARTTLS, PLAIN/LOGIN |
| SMTP client | `tests/smtp_client.rs` | 6 | End-to-end client behaviour and bad certificate fixtures |
| MIME bodies | `tests/mime_bodies.rs` | 6 | Body composition against golden fixtures |
| MIME conformance | `tests/mime_conformance.rs` | 4 | Independent parser agreement (mail-parser) |
| Leak probes | `tests/leaks.rs` | 4 | Descriptor stability across success, refusal, TLS failure, mixed stress |
| Facade | `ts/test/*.test.ts` | 44 | Public API, error classes, results, types, attachment budget |
| **Total** | | **282** | |

Two consecutive workspace runs reported **238 Rust tests passed, 0 failed** each, and the facade suite reported **44 passed** both times.

## 2. Module coverage map

| Module | Unit tests | Exercised by integration suites |
| --- | --- | --- |
| `address` | 14 | session, faults (envelope addresses) |
| `attachments` | 25 | delivery, faults, leaks (file reads) |
| `config` | 14 | faults (timeout boundaries), facade (limits) |
| `delivery` | 10 | session, faults, mime bodies |
| `errors` | 7 | every suite (codes asserted), facade error tests |
| `lifecycle` | 7 | facade close/busy tests |
| `message` | 22 | mime bodies, mime conformance, faults |
| `mime` (+ `headers`) | 28 + 10 | mime bodies, conformance, delivery |
| `results` | 8 | session, faults, facade results tests |
| `validation` | 6 | message/mime suites (precedence) |
| `transports/smtp` (mod, session) | 6 + 0 | session, faults, client |
| `proto/command`, `response`, `capabilities`, `transcript` | 5 + 9 + 2 + 2 | session, faults, harness self-test |
| `client/connection` | 2 | session, faults |
| `client/tls` | 0 | client, faults (certificate matrix) |

## 3. Critical branches and the test that covers each

| Branch | Test |
| --- | --- |
| Untrusted root, self-signed, expired, wrong host, incomplete chain, malformed anchor, malformed server certificate | `every_bad_certificate_fixture_fails_before_mail`, `fails_closed_for_each_bad_certificate_fixture`, `rejects_a_malformed_trust_anchor`, `an_untrusted_certificate_fails_closed`, `untrusted_clients_cannot_complete_the_handshake` |
| STARTTLS advertised vs not | `upgrades_with_starttls_and_rehandshakes`, `starttls_is_required_and_fails_closed_without_the_extension`, `refuses_starttls_when_not_advertised` |
| AUTH PLAIN, LOGIN, unsupported, forced-not-advertised, `535`, reset, stall, before TLS | `authenticates_with_plain`, `authenticates_with_login_challenges`, `auth_without_a_supported_mechanism_is_unsupported`, `auth_without_a_mechanism_is_unsupported`, `an_explicit_mechanism_must_be_advertised`, `auth_failure_is_an_error_and_never_mails`, `auth_reset_is_a_protocol_error_at_the_auth_stage`, `auth_stall_is_an_operation_timeout`, `auth_before_tls_is_refused` |
| Sender refused | `a_refused_sender_is_a_rejected_outcome` |
| RCPT refused, mixed, 421, intermediate reply, stall, reset | `all_recipients_refused_never_sends_data`, `mixed_recipient_replies_are_served_per_rcpt`, `a_421_before_any_rcpt_leaves_every_recipient_unknown`, `an_intermediate_rcpt_reply_is_a_protocol_error`, `rcpt_stall_is_an_operation_timeout` |
| DATA command failure vs body outcome | `data_command_reset_is_a_protocol_error`, `data_command_stall_is_an_operation_timeout`, `explicit_final_data_rejection_is_a_rejected_outcome`, `missing_final_reply_after_data_is_unknown`, `final_reply_stall_after_data_is_unknown` |
| Greeting malformed, delayed, reset | `a_malformed_greeting_is_a_protocol_error`, `greeting_reset_is_a_protocol_error`, `injected_greeting_delay_is_observable` |
| Connect refused | `connection_refused_is_a_protocol_error`, `refused_connection_is_a_protocol_error` |
| Timeout exactness and interaction | `connect_boundary_is_exact`, `operation_boundary_is_exact_and_capped_by_the_total`, `total_boundary_is_exact`, `total_stall_is_a_total_timeout` |
| Close drain, drain timeout, concurrent close, send during close | `close_drains_an_active_send`, `close_drain_times_out_without_the_active_send_finishing`, facade "closes concurrently without drifting past closed", "refuses a send that starts during close" |
| BUSY with no I/O | `a_second_send_is_busy_and_dials_nothing`, facade "rejects a second concurrent send with TRANSPORT_BUSY" |
| Caps: per-file, aggregate, wire | `enforces_the_per_file_cap_inclusively`, `enforces_the_aggregate_cap`, `wire_cap_is_inclusive_and_exact`, `preflight_rejects_bodies_that_only_fit_separately` |
| Special files and growth | `rejects_a_symlink_to_a_regular_file`, `rejects_a_fifo_without_blocking`, `rejects_a_character_device`, `rejects_a_directory`, `rejects_a_file_that_outgrows_the_cap`, `a_symlinked_file_never_reaches_the_port` |
| Injection | `rejects_header_injection_in_values`, `rejects_subject_injection`, `rejects_reserved_custom_headers`, `rejects_header_injection_in_address_and_display_name`, `rejects_invalid_filenames_and_content_types` |
| Buffer copy and detachment | facade "copies a buffer attachment before the send returns", "snapshots a buffer before the caller detaches it" |
| Redaction | `every_code_renders_without_a_secret_marker`, `auth_failure_is_an_error_and_never_mails`, `errors_never_carry_the_path`, `redacts credentials, bodies, and paths from every error` |
| Error precedence | `earliest_stage_wins_regardless_of_evaluation_order`, `every_adjacent_stage_pair_prefers_the_earlier_stage` |

## 4. Core tests run without Node

```text
env PATH=$HOME/.cargo/bin:/usr/bin:/bin sh -c 'command -v node || echo NO-NODE; \
  cargo test -p sendust-core --locked --lib --tests'
```

Result: `NO-NODE`, then **238 passed, 0 failed**. The core suite needs no JavaScript runtime, which is the C-01 boundary in executable form.

## 5. Binding semantic parity

The binding contains no policy: `lib.rs` maps fields (`build_message`, `attachment`, `to_result`, `to_napi`) and enforces the attached limits through the core. Parity is asserted where the semantics are observable:

- `ts/test/errors.test.ts` "covers every stable code with its category and class" walks the documented codes and fails if a category or class drifts.
- `ts/test/results.test.ts` "maps accepted without adding a delivery claim", "keeps rejected and unknown verbatim" assert the result mapping does not add or drop meaning.
- `ts/test/types.test.ts` "keeps the option and message surfaces exactly as documented" locks the public shape.

## 6. Skipped tests

The facade's 20 addon-dependent tests use `it.skipIf(!available)`, which skips only when the staged `sendust.node` is missing (a local checkout that has not run `npm run build:native`). CI always builds the addon, and the local run executed all 22 transport tests — no test is skipped in the gate environment. Owner: P12-002.

## 7. Not covered, and why

| Gap | Disposition |
| --- | --- |
| Line/branch coverage percentages | Not collected. No coverage tool is in the pinned toolchain; adding `cargo-llvm-cov` or `cargo-tarpaulin` is a dependency decision for the project owner, not something this task may introduce |
| `client/tls.rs` unit tests | Exercised end-to-end by the certificate matrix (`smtp_client`, `smtp_faults`); the parser's error paths are covered by `rejects_a_malformed_trust_anchor` |
| `session.rs` unit tests | No unit tests; 6 integration tests drive it directly |
| `ports/` and the `lib.rs` facade | Compile-time contracts (trait and re-exports); no runtime behaviour to test |
| Binding `escape()` | Covered indirectly by the facade redaction test; no direct unit test |
| Examples | Not tested; exercised manually against a fake relay |
| `#[cfg(not(unix))]` open fallback and Windows-specific branches | Not executable on this host; the unix path is what CI runs |
| Forced GC | Accepted as R-05 (PHASE-11) and deferred to P12-004 |

## 8. Dispositions

No product or fixture failure occurred in either run. No test was skipped in the gate environment. The gaps above are acceptance boundaries or owner decisions, not defects, and none is high severity.
