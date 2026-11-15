# Requirements-to-Test Traceability Matrix (P12-001)

**Status:** reviewed — every mandatory requirement is owned
**Date:** 2026-11-15
**Method:** each row names the requirement, the artifact that enforces it, the test file, a specific test, and the pass condition. Rows are read from `docs/product/requirements.md` (C-xx, FR-xxx, NFR-xxx), the abuse cases from [the threat-model closure](../security/threat-model-closure.md) (AB-xx), plus the limits, lifecycle, and error-code contracts. A requirement is **owned** when its row names an automated test or an approving gate. Nothing here redefines a requirement; the linked document does.

Test paths are relative to the repository root. `core tests` are Rust integration tests under `crates/sendust-core/tests/`; unit tests live beside their module under `crates/sendust-core/src/`.

## 1. Constraints (C-01…C-40)

| ID | Requirement | Enforced by | Test / evidence | Pass condition |
| --- | --- | --- | --- | --- |
| C-01 | Exactly two crates, core free of Node | `scripts/check-architecture.mjs` | `checkWorkspaceShape` | Checker exits 0; the workspace names exactly `sendust-core` and `sendust-napi` |
| C-02 | Dependency direction TS → napi → core | `check-architecture` | `checkCoreModules` layering + `checkAdapterBoundaries` | No upward or cross-layer import; the checker passes |
| C-03 | Sendust owns its SMTP engine, no third-party engine | `check-architecture` | `code.includes("lettre::")` rule; `cargo tree` | No `lettre` in the tree or in source |
| C-04 | One Tokio runtime, no blocking I/O on the reactor | Code review + lockfile | `crates/sendust-napi/src/lib.rs` (`tokio::spawn` only); `Cargo.lock` has one `tokio` | One runtime; file reads happen in the core task, never on a JS thread |
| C-05 | Node-API 8 with `napi8` + `async` | `crates/sendust-napi/Cargo.toml` | CI job *Native binding (Node.js 22)* | Binding loads and passes its suite |
| C-06 | Support claims match CI | CI matrix | `.github/workflows/ci.yml` Node 22/24/26, Bun, Deno jobs | Only tested combinations are declared |
| C-07 | TLS verification always on, TLS 1.2 floor | P11-002 | `tests/smtp_faults.rs::every_bad_certificate_fixture_fails_before_mail`; `check-architecture` bypass markers | Every fixture fails before `MAIL`; the checker rejects any bypass marker |
| C-08 | At most one active send, immediate `BUSY` | Lifecycle | `src/lifecycle/mod.rs::a_second_send_is_busy_and_dials_nothing`; `ts/test/transport.test.ts` "rejects a second concurrent send with TRANSPORT_BUSY" | Second call rejects with `TRANSPORT_BUSY` and dials nothing |
| C-09 | Explicit, idempotent, bounded lifecycle | Lifecycle + facade | `close_is_idempotent_and_refuses_new_sends`, `send_after_close_is_refused`, `close_drain_times_out_without_the_active_send_finishing`; facade "closes idempotently and refuses later sends" | States only move forward; close is bounded; post-close sends fail with `TRANSPORT_CLOSED` |
| C-10 | Finite, capped timeout budgets | Config | `src/config/mod.rs::connect_boundary_is_exact`, `operation_boundary_is_exact_and_capped_by_the_total`, `total_boundary_is_exact`, `rejects_raised_zero_sub_second_and_inconsistent_timeouts` | Defaults 10/30/120/125 s; raising, zero, or inconsistent values rejected |
| C-11 | Owned memory before any await | Binding + facade | facade "copies a buffer attachment before the send returns", "snapshots a buffer before the caller detaches it"; `attachment()` in `lib.rs` uses `to_vec()` | Caller mutation has no effect on the send |
| C-12 | Filesystem attachments opt-in, regular files only | P11-003 | `src/attachments/mod.rs::filesystem_access_is_disabled_by_default`, `rejects_file_attachments_without_opt_in`, `rejects_a_symlink_to_a_regular_file`, `rejects_a_fifo_without_blocking`, `rejects_a_character_device`, `rejects_a_directory`; `src/delivery/mod.rs::a_symlinked_file_never_reaches_the_port` | Path attachments fail without opt-in; special files never reach the port |
| C-13 | Hard caps, never raisable | P11-003 | `src/attachments/mod.rs::enforces_the_per_file_cap_inclusively`, `enforces_the_aggregate_cap`; `src/mime/mod.rs::wire_cap_is_inclusive_and_exact`; `src/config/mod.rs::limits_cannot_be_raised_or_zeroed`; `ts/src/limits.ts` parity check | Over-cap input fails before DATA; caps cannot be raised |
| C-14 | ASCII local part, IDNA domain, Unicode display/body | Address + MIME | `rejects_non_ascii_local_part`, `converts_idna_domain_to_ascii`, `keeps_unicode_display_name_and_quotes_it_safely`; `encodes_non_ascii_subjects_and_display_names` | Accepted per policy, rejected otherwise |
| C-15 | Relay-acceptance semantics, `unknown` never retried | SMTP session + faults | `accepts_a_single_recipient_transaction`, `explicit_final_data_rejection_is_a_rejected_outcome`, `missing_final_reply_after_data_is_unknown`, `final_reply_stall_after_data_is_unknown` | Each outcome maps to exactly one final reply class; unknown is not retried |
| C-16 | No pool, retry, queue, or worker | Fault suite + code | Every timeout test asserts one `MAIL`; `check_core_modules` has no queue/worker module | One connection per send; no retry command follows a failure |
| C-17 | No public `AbortSignal` | Facade types | `ts/test/types.test.ts` "rejects options outside the frozen surface"; `TransportOptions` has no signal field | Unknown keys are rejected at runtime and absent from the type surface |
| C-18 | Recipient-level RCPT outcomes, one DATA transaction | SMTP session | `mixed_recipient_replies_share_one_data_transaction`, `all_recipients_refused_never_sends_data`, `a_421_before_any_rcpt_leaves_every_recipient_unknown`, `a_421_keeps_the_attempted_statuses_and_marks_the_rest_unknown` | Accepted subset gets one DATA; 421 sends no DATA; per-recipient statuses match the replies |
| C-19 | No secret, body, or path in diagnostics | P11-004 | `src/errors/mod.rs::every_code_renders_without_a_secret_marker`, `defaults_are_redacted_and_carry_the_code`; `tests/smtp_faults.rs::auth_failure_is_an_error_and_never_mails`; `src/attachments/mod.rs::errors_never_carry_the_path`; `ts/test/errors.test.ts` "redacts credentials, bodies, and paths from every error" | No marker appears in Display or Debug |
| C-20 | Stable codes, no panic across FFI | Errors + binding | `every_code_is_documented_and_unique`, `details_are_whitelisted_and_readable`; `#[napi(catch_unwind)]` on `new`/`state`; `Err(_) => INTERNAL` | Codes are unique and documented; a panic becomes an error |
| C-21 | Deterministic loopback-only tests | Harness + checker | `check-architecture` loopback rules; `tests/harness_self_test.rs` | No test binds `0.0.0.0` or names a URL |
| C-22 | ESM and CommonJS parity | P14-002/P14-004 | `.github/workflows/ci.yml`; `package.json` `"type": "module"` | Gate owned by P14; not claimed before it runs |
| C-23 | Reproducible, provenance-tracked release | P16 | P16-002/P16-004 | Gate owned by P16 |
| C-24 | Evidence-based completion | Every gate | This matrix; the phase records | No task marked COMPLETED without evidence |
| C-25 | Single source of truth | `check-docs.mjs` | Link and index checks | Checker exits 0 |
| C-26 | Honest support matrix | P14-003/P14-004 | Platform gate records | Gate owned by P14 |
| C-27 | No consumer toolchain | P14/P17 | Prebuilt-binary install gate | Gate owned by P14/P17 |
| C-28 | No plaintext SMTP | Binding + facade | `TransportOptions.secure` is `"tls" \| "starttls"`; facade "rejects invalid relay options with a stable code"; `src/transports/smtp/mod.rs::relay_configuration_is_validated` | Any other mode is rejected before connecting |
| C-29 | CRLF/control rejection, reserved headers locked | P11-003 | `rejects_header_injection_in_values`, `rejects_subject_injection`, `rejects_reserved_custom_headers`, `rejects_header_injection_in_address_and_display_name`, `rejects_invalid_filenames_and_content_types` | `VALIDATION_HEADER_INJECTION` / `VALIDATION_INVALID_HEADER` |
| C-30 | Standalone delivery-engine boundary | Product review | `docs/product/mvp-scope.md`; no queue/worker module exists | Manual evidence, recorded in P15-005 |
| C-31 | Node.js, Bun, and Deno required | CI | CI jobs for Node 22/24/26, Bun, Deno | All blocking jobs pass |
| C-32 | Two crates, one binding implementation | `check-architecture` | Workspace + adapter checks | Checker exits 0 |
| C-33 | Runtime-neutral shared TypeScript | `check-architecture` | `NODE_ONLY_PATTERNS` and `ANY_ESCAPE` rules | No Node-only global or `any` in shared sources |
| C-34 | Deno permissions and layout | CI | *Deno runtime-neutral smoke* job | Smoke passes with the documented import path |
| C-35 | Support is tested combinations only | CI | Matrix jobs | Declared set equals the CI set |
| C-36 | Explicit platform baseline | P14 | Platform gate records | Gate owned by P14 |
| C-37 | Modular monolith | ADR-0016 + checker | `MODULE_LAYERS` and cycle detection | Checker exits 0 |
| C-38 | Core module ownership and acyclic deps | `check-architecture` | Layer edges + `visit` cycle detection | No forbidden edge, no cycle |
| C-39 | SMTP is the only transport | `check-architecture` | Adapter containment rule; single `transports/smtp` module | Checker exits 0 |
| C-40 | Boundary enforcement with tests | `check-architecture` + unit tests | The 177 core unit tests, one suite per module | Every module has focused tests; the checker passes |

## 2. Functional requirements (FR-001…FR-068)

| ID | Requirement | Test / evidence | Pass condition |
| --- | --- | --- | --- |
| FR-001 | Relay host, port, security, EHLO, auth | `relay_configuration_is_validated`; facade "rejects invalid relay options with a stable code" | Valid config accepted, invalid rejected |
| FR-002 | Implicit TLS | `serves_an_accepted_dialogue_over_implicit_tls`, `connects_over_implicit_tls_and_reads_capabilities` | Full transaction over TLS |
| FR-003 | STARTTLS upgrade | `starttls_is_required_and_fails_closed_without_the_extension`, `upgrades_with_starttls_and_rehandshakes` | Upgrade, or `TLS_HANDSHAKE` when not advertised |
| FR-004 | AUTH PLAIN | `authenticates_with_plain`, `builds_the_plain_initial_response` | Exact challenge response and `235` |
| FR-005 | AUTH LOGIN | `authenticates_with_login_challenges`, `serializes_auth_in_both_forms` | Exact two-step response |
| FR-006 | Connect timeout | `connect_stall_is_a_connect_timeout`, `connect_boundary_is_exact` | `TIMEOUT_CONNECT` at the budget, no `MAIL` |
| FR-007 | Operation timeout | `command_stall_is_an_operation_timeout`, `auth_stall_is_an_operation_timeout`, `rcpt_stall_is_an_operation_timeout`, `data_command_stall_is_an_operation_timeout` | `TIMEOUT_OPERATION` at the stage |
| FR-008 | Total timeout | `total_stall_is_a_total_timeout`, `total_boundary_is_exact` | `TIMEOUT_TOTAL`, no retry |
| FR-009 | Parse and classify replies | `reply_classes_map_to_verdicts`, `parses_enhanced_status_fields`, `rejects_malformed_codes_and_separators` | 2xx/4xx/5xx/421 classification is exact |
| FR-010 | Verify certificates and hostnames | `every_bad_certificate_fixture_fails_before_mail`, `fails_closed_for_each_bad_certificate_fixture`, `untrusted_clients_cannot_complete_the_handshake` | `TLS_CERTIFICATE` before `MAIL` |
| FR-011 | Reject unsupported security modes and mechanisms | facade "rejects an unsupported auth method"; `an_explicit_mechanism_must_be_advertised`; `auth_without_a_mechanism_is_unsupported` | Stable validation or auth code, no send |
| FR-020 | From with optional display name | `serializes_a_plain_text_message`; facade "reports a missing From as VALIDATION_MISSING_FROM" | `From:` rendered; missing From rejected |
| FR-021 | To recipients | `parses_a_text_only_message`; `counts_recipients_across_to_cc_and_bcc` | All To addresses in the envelope and header |
| FR-022 | CC recipients | `counts_recipients_across_to_cc_and_bcc`; `parses_alternative_bodies_and_unicode_subjects` | CC in header and envelope |
| FR-023 | BCC hidden from headers | `serializes_attachments_and_never_leaks_bcc` | BCC address absent from the wire bytes |
| FR-024 | Reply-To | `emits_headers_in_the_approved_order`; `parses_a_text_only_message` | `Reply-To:` present only when supplied |
| FR-025 | Unicode subject | `encodes_non_ascii_subjects_and_display_names`, `renders_exact_encoded_word_bytes` | RFC 2047 encoded word |
| FR-026 | Plain-text body | `serializes_a_plain_text_message`, `text_only_body_matches_the_golden_fixture` | Exact body part |
| FR-027 | HTML body | `serializes_html_only`, `html_only_body_is_text_html` | `text/html` part only |
| FR-028 | Custom headers | `accepts_a_custom_header`, `includes_custom_headers`, `encodes_non_ascii_custom_header_values` | Header rendered once, encoded when needed |
| FR-029 | multipart/alternative | `serializes_alternative_bodies`, `alternative_body_matches_the_golden_fixture` | Both parts, alternative structure |
| FR-030 | multipart/mixed | `serializes_attachments_and_never_leaks_bcc`; `file_attachments_are_loaded_into_the_wire` | Attachment part under mixed |
| FR-031 | Address validation | `parses_plain_ascii_address`, `converts_idna_domain_to_ascii`, `rejects_malformed_addresses` | Accepted or `VALIDATION_INVALID_ADDRESS` |
| FR-032 | CRLF/control rejection | `rejects_header_injection_in_values`, `rejects_invalid_filenames_and_content_types`, `envelope_rejects_addresses_the_codec_would_misread` | Injection codes, no send |
| FR-033 | CRLF line endings and folding | `every_line_is_within_the_rfc_limit`, `line_endings_normalize_to_crlf_before_encoding`, `folds_unbreakable_header_values_within_the_line_limit` | Every line ≤ 998 octets, CRLF only |
| FR-034 | Reserved headers protected | `rejects_reserved_custom_headers`, `rejects_a_custom_message_id_or_received_header`, `rejects_a_duplicate_custom_header_case_insensitively` | `VALIDATION_INVALID_HEADER` |
| FR-035 | Encoded display names and subjects | `encodes_non_ascii_subjects_and_display_names`, `encodes_unicode_display_names_without_quoting` | RFC 2047 encoding |
| FR-040 | Buffer attachments | facade "copies a buffer attachment before the send returns"; `accepts_a_buffer_attachment` | Bytes rendered base64 |
| FR-041 | Files only with opt-in | `rejects_file_attachments_without_opt_in`; facade "rejects a path attachment without filesystem opt-in" | `CONFIG_FILE_ACCESS_DISABLED` |
| FR-042 | Bounded regular files | `rejects_a_file_that_outgrows_the_cap`, `rejects_a_directory`, `rejects_a_symlink_to_a_regular_file`, `rejects_a_fifo_without_blocking`, `rejects_a_character_device` | `CONFIG_ATTACHMENT_UNREADABLE` / `LIMIT_ATTACHMENT_BYTES` |
| FR-043 | Safe filename encoding | `strips_paths_down_to_the_display_name`, `strips_a_path_from_the_rendered_filename`, `encodes_a_unicode_filename_per_rfc2231` | Stripped name, escaped quotes, RFC 2231 |
| FR-044 | Content type resolution | `infers_a_content_type_from_the_extension`, `infers_an_attachment_content_type_from_the_filename` | Explicit, extension, or `application/octet-stream` |
| FR-045 | Caps before DATA | `enforces_the_per_file_cap_inclusively`, `enforces_the_aggregate_cap`, `wire_cap_is_inclusive_and_exact`, `preflight_rejects_bodies_that_only_fit_separately` | Limit codes; port never called |
| FR-046 | No URL/stream/directory/glob attachments | `check-architecture` URL rule; `Attachment` type has only `Bytes`/`File`; `ts/test/types.test.ts` "limits attachments to buffer and path shapes" | No such shape exists |
| FR-050 | One message per call | `submits_once_with_serialized_wire_bytes`; facade "validates a message before opening a connection" | Exactly one transaction |
| FR-051 | Up to 100 recipients | `recipient_cap_is_inclusive` | 100 accepted, 101 rejected with `LIMIT_RECIPIENTS` |
| FR-052 | Structured result | `accepted_reports_relay_acceptance`, `unknown_carries_the_ambiguity_stage_and_no_response`; `ts/test/results.test.ts` "maps accepted without adding a delivery claim" | Status and recipients match the contract |
| FR-053 | Honest recipient failure | `rejected_keeps_per_recipient_detail`, `mixed_recipient_replies_are_served_per_rcpt` | Per-recipient status preserved |
| FR-054 | BUSY with no I/O | `a_second_send_is_busy_and_dials_nothing`; facade "rejects a second concurrent send with TRANSPORT_BUSY" | `TRANSPORT_BUSY`, one connection |
| FR-055 | Explicit bounded close | `close_drains_an_active_send`, `close_drain_times_out_without_the_active_send_finishing`; facade "closes a stalled send within the close budget" | Close resolves or `TIMEOUT_CLOSE` |
| FR-056 | Stable error after close | `send_after_close_is_refused`; facade "closes idempotently and refuses later sends" | `TRANSPORT_CLOSED` |
| FR-057 | Never retry, never per-recipient | Every fault test asserts one `MAIL`; `mixed_recipient_replies_share_one_data_transaction` | One transaction per call |
| FR-060 | TypeScript declarations | `npm --workspace ./ts run typecheck`; `ts/test/types.test.ts` | `tsc` exits 0 against the documented surface |
| FR-061 | ESM import | P14-004 | Gate owned by P14 |
| FR-062 | CommonJS require | P14-004 | Gate owned by P14 |
| FR-063 | Install without a toolchain | P14-004/P17 | Gate owned by P14/P17 |
| FR-064 | Stable classes and codes | `ts/test/errors.test.ts` "covers every stable code with its category and class", "keeps the class hierarchy narrow enough to catch one category" | Every code maps to one class |
| FR-065 | Runnable examples | `crates/sendust-core/examples/send_email.rs`, `examples/send_email.mjs` | Both run against a fake relay (manual evidence in P15) |
| FR-066 | Documentation set | P15-001…P15-005 | Gate owned by P15 |
| FR-067 | Reject unknown option keys | facade "rejects unknown option keys before touching the relay"; `ts/test/types.test.ts` "rejects options outside the frozen surface" | `VALIDATION_INVALID_OPTION` before any network |
| FR-068 | Export the package version | facade "exposes the package version" | `version` is a string |

## 3. Non-functional requirements (NFR-001…NFR-014)

| ID | Requirement | Test / evidence | Pass condition |
| --- | --- | --- | --- |
| NFR-001 | TLS with no bypass | P11-002; `every_bad_certificate_fixture_fails_before_mail`; `check-architecture` markers | No bypass path exists |
| NFR-002 | Injection resistance | P11-003; the CRLF/NUL/reserved-header fixtures | Injection codes, no send |
| NFR-003 | No secret/body/path leakage | P11-004; `every_code_renders_without_a_secret_marker`, `errors_never_carry_the_path`, `auth_failure_is_an_error_and_never_mails` | No marker in Display or Debug |
| NFR-004 | Bounded memory | P11-003; the cap tests and the facade pre-flight check | No unbounded allocation before a cap |
| NFR-005 | Performance budget | P13-001…P13-005 | Gate owned by P13 |
| NFR-006 | Cross-platform packaging | P14-003/P14-004 | Gate owned by P14 |
| NFR-007 | Deterministic tests | `check-architecture` loopback rules; `tests/harness_self_test.rs` | No external network, no `0.0.0.0` |
| NFR-008 | No sensitive logging | P11-004; no logging crate in `Cargo.toml` | Nothing is logged |
| NFR-009 | Panic safety | `#[napi(catch_unwind)]`; `Err(_) => INTERNAL`; no `unwrap`/`expect` in production code | No panic crosses the boundary |
| NFR-010 | No `any` in public results/errors | `check-architecture` `ANY_ESCAPE`; `npm run typecheck` | Checker and `tsc` pass |
| NFR-011 | Reproducible release artifacts | P16-002/P17-001 | Gate owned by P16/P17 |
| NFR-012 | Documentation accuracy | P15-005/P17-002 | Gate owned by P15/P17 |
| NFR-013 | Minimal licensed dependencies | P11-005 licence inventory; `check-docs` pin checks | Exact pins; permissive licences only |
| NFR-014 | Deterministic error precedence | `earliest_stage_wins_regardless_of_evaluation_order`, `every_adjacent_stage_pair_prefers_the_earlier_stage`, `failures_in_the_same_stage_keep_the_first_recorded` | The earliest stage always wins |

## 4. Limits, lifecycle, and error codes

| Contract | Value / rule | Test | Pass condition |
| --- | --- | --- | --- |
| Recipients per message | 100 | `recipient_cap_is_inclusive`; facade "enforces the aggregate cap across path attachments" (cap path) | 100 accepted, 101 `LIMIT_RECIPIENTS` |
| Per-attachment bytes | 10 MiB | `enforces_the_per_file_cap_inclusively`; facade "rejects an oversize attachment before copying it" | Cap inclusive; one byte over rejected |
| Aggregate attachment bytes | 20 MiB | `enforces_the_aggregate_cap`; `ts/test/limits.test.ts` "rejects the aggregate when every file on its own fits" | Cap inclusive |
| Wire message | 30 MiB | `wire_cap_is_inclusive_and_exact`, `enforces_the_default_wire_cap_on_large_bodies` | Cap inclusive, exact |
| Active sends per transport | 1 | `a_second_send_is_busy_and_dials_nothing` | Second send is `TRANSPORT_BUSY` |
| Lifecycle order | open → closing → closed | `close_is_idempotent_and_refuses_new_sends`, `closes concurrently without drifting past closed` | No backward transition |
| Close deadline | 125 s | `close_drain_times_out_without_the_active_send_finishing` | `TIMEOUT_CLOSE` at the budget |
| Error codes | Unique, documented, categorised | `every_code_is_documented_and_unique`, `categories_expose_stable_strings`, `stages_expose_stable_strings`; `ts/test/errors.test.ts` | Enum, docs, and classes agree |

## 5. Security abuse cases (AB-01…AB-14)

The closure matrix in [the threat-model closure](../security/threat-model-closure.md) already maps each case to its control and test; this table names the primary test so the QA package stands alone.

| ID | Primary evidence | State |
| --- | --- | --- |
| AB-01 | `starttls_is_required_and_fails_closed_without_the_extension`, `refuses_starttls_when_not_advertised` | CLOSED |
| AB-02 | `every_bad_certificate_fixture_fails_before_mail`, `fails_closed_for_each_bad_certificate_fixture` | CLOSED |
| AB-03 | `rejects_header_injection_in_values`, `rejects_subject_injection`, `rejects_invalid_filenames_and_content_types` | CLOSED |
| AB-04 | `recipient_cap_is_inclusive`, `enforces_the_aggregate_cap`, `wire_cap_is_inclusive_and_exact` | CLOSED |
| AB-05 | `rejects_a_symlink_to_a_regular_file`, `rejects_a_fifo_without_blocking`, `rejects_a_character_device`, `rejects_a_directory` | CLOSED |
| AB-06 | `rejects_a_file_that_outgrows_the_cap` | CLOSED |
| AB-07 | facade "snapshots a buffer before the caller detaches it" | CLOSED |
| AB-08 | `connect_boundary_is_exact`, `operation_boundary_is_exact_and_capped_by_the_total`, `total_boundary_is_exact` | CLOSED |
| AB-09 | `missing_final_reply_after_data_is_unknown`, `final_reply_stall_after_data_is_unknown` | CLOSED |
| AB-10 | `a_second_send_is_busy_and_dials_nothing`, facade "refuses a send that starts during close" | CLOSED |
| AB-11 | `#[napi(catch_unwind)]`; `Err(_) => INTERNAL`; no `unwrap` in production | CLOSED |
| AB-12 | `every_code_renders_without_a_secret_marker`, `auth_failure_is_an_error_and_never_mails` | CLOSED |
| AB-13 | `check-architecture` loopback rules; `scripts/check-secrets.mjs` | CLOSED |
| AB-14 | Not implemented in code | DEFERRED — P14-003/P16-002 |

## 6. Gaps and dispositions

| Gap | Owner | Disposition |
| --- | --- | --- |
| ESM/CommonJS parity and clean-install claims (C-22, FR-061–FR-063) | P14-004 | Deferred by design; no support claim is made until the gate runs |
| Platform matrix and provenance (C-23, C-26, C-36, NFR-006, NFR-011) | P14/P16/P17 | Deferred by design; gated in those phases |
| Documentation accuracy (C-30, FR-066, NFR-012) | P15/P17 | Deferred to the documentation gate |
| No forced-GC automation | P12-004 | Deferral accepted as R-05 in the PHASE-11 record; nothing finalizer-driven exists |
| No DNS-failure fixture | Accepted R-03 | Accepted; the connect deadline and reset paths are covered |
| Single-runtime claim (C-04) | Static review | One `tokio` in `Cargo.lock`, one `tokio::spawn` call site; no second runtime exists |

Every mandatory requirement above is owned by an automated test or an approving gate. The only deferrals are owned by later phases with their own gate records, and each is recorded as an accepted risk rather than an unowned requirement.
