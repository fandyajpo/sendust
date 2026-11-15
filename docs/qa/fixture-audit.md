# Fixture Completeness and Regression Triage (P12-004)

**Status:** complete — every required fixture category is owned; two gaps closed in this task
**Date:** 2026-11-15
**Method:** the checklist below was audited against the Phase 00 threat model, the Phase 10 fault matrix, and the Phase 11 audits; missing cases were added (a forced-GC probe), nondeterministic sleeps were removed from the facade suite, and the full suite was run twice. Every row names the test that owns the fixture.

## 1. Fixture checklist

| Category | Fixtures | Owning tests | Result |
| --- | --- | --- | --- |
| Success | all recipients accepted; one DATA transaction | `accepts_a_single_recipient_transaction`, `mixed_recipient_replies_share_one_data_transaction` | PASS |
| Rejection | every RCPT refused; explicit final DATA `451`; refused sender | `all_recipients_refused_never_sends_data`, `explicit_final_data_rejection_is_a_rejected_outcome`, `a_refused_sender_is_a_rejected_outcome` | PASS |
| Mixed RCPT | `250`/`550` mix; intermediate reply; `421` before and after some RCPT | `mixed_recipient_replies_are_served_per_rcpt`, `an_intermediate_rcpt_reply_is_a_protocol_error`, `a_421_before_any_rcpt_leaves_every_recipient_unknown`, `a_421_keeps_the_attempted_statuses_and_marks_the_rest_unknown` | PASS |
| Unknown | missing final reply; final-reply stall; connection drop after terminator; cancellation after DATA | `missing_final_reply_after_data_is_unknown`, `final_reply_stall_after_data_is_unknown`, `connection_dropped_after_terminator_has_no_final_reply`, `a_cancellation_after_data_reports_unknown_for_the_accepted_subset` | PASS |
| TLS | self-signed, expired, wrong host, incomplete chain, untrusted root, malformed anchor, malformed server certificate, refused handshake | `every_bad_certificate_fixture_fails_before_mail`, `fails_closed_for_each_bad_certificate_fixture`, `an_untrusted_certificate_fails_closed`, `rejects_a_malformed_trust_anchor`, `untrusted_clients_cannot_complete_the_handshake` | PASS |
| Auth | PLAIN, LOGIN, `535`, unsupported mechanism, forced-not-advertised, AUTH reset, AUTH stall, AUTH before TLS | `authenticates_with_plain`, `authenticates_with_login_challenges`, `auth_failure_is_an_error_and_never_mails`, `auth_without_a_supported_mechanism_is_unsupported`, `an_explicit_mechanism_must_be_advertised`, `auth_reset_is_a_protocol_error_at_the_auth_stage`, `auth_stall_is_an_operation_timeout`, `auth_before_tls_is_refused` | PASS |
| Resets | greeting, MAIL/command, AUTH, DATA command | `greeting_reset_is_a_protocol_error`, `command_stall_is_an_operation_timeout` (reset counterpart `data_command_reset_is_a_protocol_error`), `auth_reset_is_a_protocol_error_at_the_auth_stage` | PASS |
| Stalls | greeting, connect, AUTH, RCPT, DATA command, final reply, total | `connect_stall_is_a_connect_timeout`, `auth_stall_is_an_operation_timeout`, `rcpt_stall_is_an_operation_timeout`, `data_command_stall_is_an_operation_timeout`, `final_reply_stall_after_data_is_unknown`, `total_stall_is_a_total_timeout` | PASS |
| Close | idempotent, concurrent, drain, drain timeout, send during close, in-flight abort | `close_is_idempotent_and_refuses_new_sends`, `closes concurrently without drifting past closed`, `close_drains_an_active_send`, `close_drain_times_out_without_the_active_send_finishing`, `refuses a send that starts during close`, `aborts a send in flight and rejects it as TRANSPORT_CLOSED` | PASS |
| Limits | 100/101 recipients; 10 MiB per file; 20 MiB aggregate; 30 MiB wire; lowered caps | `recipient_cap_is_inclusive`, `enforces_the_per_file_cap_inclusively`, `enforces_the_aggregate_cap`, `wire_cap_is_inclusive_and_exact`, `honours a lowered attachment cap in the facade` | PASS |
| Attachments | buffer, file, symlink, FIFO, device, directory, missing, permission denied, growth, wire accounting | `reads_a_regular_file`, `rejects_a_symlink_to_a_regular_file`, `rejects_a_fifo_without_blocking`, `rejects_a_character_device`, `rejects_a_directory`, `rejects_a_missing_path`, `reports_an_unreadable_file_as_unreadable`, `rejects_a_file_that_outgrows_the_cap`, `the_wire_cap_counts_attachment_encoding` | PASS |
| Injection | CRLF, bare LF, bare CR, NUL, reserved headers, header-like display name, non-ASCII local part, IDNA, path-like filename | `rejects_header_injection_in_values`, `rejects_subject_injection`, `rejects_reserved_custom_headers`, `rejects_header_injection_in_address_and_display_name`, `rejects_header_like_display_names`, `rejects_non_ascii_local_part`, `converts_idna_domain_to_ascii`, `strips_a_path_from_the_rendered_filename` | PASS |
| Buffer mutation | copy before return; detached buffer; pre-copy cap | `copies a buffer attachment before the send returns`, `snapshots a buffer before the caller detaches it`, `rejects an oversize attachment before copying it` | PASS |
| GC | 50 collected transports; abandoned open transport; later transport unaffected | `tests/runtime/gc.mjs` (new) | PASS |
| BUSY | second send rejected with one connection | `a_second_send_is_busy_and_dials_nothing`, `rejects a second concurrent send with TRANSPORT_BUSY` | PASS |

## 2. Changes made in this task

| Change | Detail |
| --- | --- |
| Forced-GC fixture added | `tests/runtime/gc.mjs` runs under `node --expose-gc`, asserts the addon survives 50 collected transports, that an abandoned open transport is never closed by the collector, and that a later transport still works. Added to the native binding CI job and to the toolchain command matrix. Closes R-05. |
| Facade sleeps removed | The four `setTimeout`/`delay` waits in `transport.test.ts` are replaced by a `connectionGate()` promise that resolves when the fake relay accepts its connection. The suite now sequences on events, not wall-clock guesses, and the run time dropped from ~400 ms to ~200 ms. |
| Rust sleeps reviewed | The remaining `thread::sleep` calls are deliberate fault injections (`tests/support/mod.rs` scripted delays) and a bounded close-drain fixture (`lifecycle` test), not test sequencing; they stay. |

## 3. Regression triage

| Run | Rust | Facade | GC |
| --- | --- | --- | --- |
| 1 | 238 passed, 0 failed | 44 passed | 5 checks passed |
| 2 | 238 passed, 0 failed | 44 passed | 5 checks passed |

No failure occurred in either run, so no defect needed a disposition. No test is skipped in the gate environment: the 20 addon-dependent facade tests skip only when `sendust.node` is missing locally.

## 4. Open gaps

| Gap | Disposition |
| --- | --- |
| No JS end-to-end `accepted`/`rejected`/`unknown` fixture | The public API accepts no trust anchor (C-07), so a loopback TLS relay cannot be trusted. Mapping and core semantics are covered separately. Reassigned from P12 QA to P14-004, where an anchor policy or a public-CA sandbox can be decided; the threat-model residual R-04 is updated accordingly |
| Line coverage | Not collected; no coverage tool is pinned (P12-002 record) |
| Forced GC on Bun/Deno | Those runtimes have no `--expose-gc` equivalent in the pinned CLI; the Node.js binding job owns the probe |

No fixture is unowned and no gap is high severity.
