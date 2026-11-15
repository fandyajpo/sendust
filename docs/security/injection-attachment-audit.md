# Validation, MIME, and Attachment Security Audit (P11-003)

**Status:** audited — one low finding, fixed in this task
**Date:** 2026-11-15
**Scope:** address and IDNA parsing, header and subject validation, filename handling and path traversal, MIME rendering, attachment reads, the filesystem default, the absence of any URL fetch path, and where the caps are enforced.

## 1. Injection fixtures

| Input | Rule | Result | Evidence |
| --- | --- | --- | --- |
| CRLF in a header value, subject, display name, or local part | No control character may appear in a header value, subject, or address | `VALIDATION_HEADER_INJECTION`, no socket | `rejects_header_injection_in_values`, `rejects_subject_injection`, `rejects_header_injection_in_address_and_display_name` |
| Bare LF (`\n`) | Same rule: LF is a control character | `VALIDATION_HEADER_INJECTION` | `rejects_header_injection_in_values` (`"value\nBcc: …"`) |
| Bare CR, vertical tab | Same rule | `VALIDATION_HEADER_INJECTION` | `rejects_header_injection_in_values` |
| NUL in a header value or filename | Same rule | `VALIDATION_HEADER_INJECTION` | `rejects_header_injection_in_values`, `rejects_invalid_filenames_and_content_types` |
| Tab in a header value | Allowed, and only inside the value | Accepted | `allows_tab_inside_values` |
| Header-like name (`Bcc`, `Subject`, `Content-Type`, `Return-Path`, `Message-ID`, `Received`) as a custom header | The reserved list cannot be overridden or shadowed | `VALIDATION_INVALID_HEADER` | `rejects_reserved_custom_headers`, `rejects_a_custom_message_id_or_received_header` |
| Duplicate custom header, any case | One value per header | `VALIDATION_INVALID_HEADER` | `rejects_a_duplicate_custom_header_case_insensitively` |
| Display name containing a colon or a header-looking string | Quoted or rejected; never emitted as a bare header | Quoted, or `VALIDATION_HEADER_INJECTION` | `rejects_header_like_display_names`, `accepts_quoted_display_name_with_colon` |
| Non-ASCII local part | Local part is ASCII only; Unicode belongs to the display name and domain | `VALIDATION_INVALID_ADDRESS` | `rejects_non_ascii_local_part` |
| IDNA domain | Converted with `idna` (UTS #46), then lowercased | Accepted as A-label | `converts_idna_domain_to_ascii`, `lowercases_ascii_domains_and_keeps_punycode` |
| Overlong name, value, address part, or header count | Documented caps: 78/998 octets, address part limits | Rejected | `rejects_overlong_values`, `rejects_overlong_parts` |
| SMTP command argument containing a space or control byte | The codec refuses anything it would misread | `PROTOCOL_*`, connection closed | `rejects_injection_and_whitespace_in_arguments` |

## 2. Filenames and paths

| Case | Result | Evidence |
| --- | --- | --- |
| `../../secrets/notes.txt`, `/etc/passwd`, `dir\report.pdf` | Reduced to the last component for display; the path never reaches a header | `strips_paths_down_to_the_display_name`, `strips_a_path_from_the_rendered_filename` |
| `.`, `..`, empty, whitespace-only, over 255 bytes | `VALIDATION_INVALID_ATTACHMENT` | `rejects_invalid_filenames_and_content_types`, `rejects_filenames_over_the_length_limit` |
| Control characters in a filename | `VALIDATION_HEADER_INJECTION` | `rejects_invalid_filenames_and_content_types` |
| Duplicate display names, case-insensitive | `VALIDATION_INVALID_ATTACHMENT` | `rejects_duplicate_filenames_case_insensitively` |
| Relative, empty, over 1024-byte, or control-bearing file path | `VALIDATION_INVALID_ATTACHMENT` / `VALIDATION_HEADER_INJECTION` before any open | `rejects_relative_or_malformed_paths` |
| Unicode filename in `Content-Disposition` | `filename*` per RFC 2231 plus an ASCII `filename` fallback; quotes escaped | `encodes_a_unicode_filename_per_rfc2231`, `content_disposition` / `escape_quotes` |

## 3. MIME rendering

- Header order, folding, and encoded words are deterministic and every rendered line stays within the RFC 5322 limit (`every_line_is_within_the_rfc_limit`, `folds_at_spaces_and_unfolds_to_the_original_value`).
- Bodies are normalized to CRLF and every part is base64-encoded, so body content cannot terminate a part or reach a header (`normalizes_lf_and_bare_cr_to_crlf`, `base64_matches_known_vectors`).
- BCC never appears in the wire bytes (`serializes_attachments_and_never_leaks_bcc`).
- Attachment `Content-Type` comes from the caller, then a fixed documented extension map, then `application/octet-stream`; a caller value is token-validated (`normalizes_content_type`).
- The wire cap is checked during rendering and again on the final byte count; a message whose parts only fit separately is rejected before the port is called (`preflight_rejects_bodies_that_only_fit_separately`, `wire_cap_is_inclusive_and_exact`).

## 4. Attachment reads

| Fixture | Result | Evidence |
| --- | --- | --- |
| Symlink to a regular file | `CONFIG_ATTACHMENT_UNREADABLE`, before the port | `rejects_a_symlink_to_a_regular_file`, `a_symlinked_file_never_reaches_the_port` |
| FIFO | Refused without blocking (`O_NONBLOCK` plus a pre-open type check) | `rejects_a_fifo_without_blocking` |
| Character device (`/dev/null`) | Refused | `rejects_a_character_device` |
| Directory | Refused | `rejects_a_directory` |
| Missing file, permission denied | `CONFIG_ATTACHMENT_UNREADABLE` | `rejects_a_missing_path`, `reports_an_unreadable_file_as_unreadable` |
| File that grows past the cap | `LIMIT_ATTACHMENT_BYTES`; the reader stops one byte past the cap and never returns a prefix | `rejects_a_file_that_outgrows_the_cap` |
| Over-limit file or aggregate at send time | No dial, no partial transmission | `an_oversized_file_never_reaches_the_port`, `an_aggregate_over_the_limit_never_reaches_the_port` |
| Path echoed in an error | Never present in Display or Debug | `errors_never_carry_the_path` |
| File handle after a read | Closed (the file can be deleted on every platform) | `the_file_handle_is_closed_after_a_read` |
| TOCTOU between the path check and the open | `O_NOFOLLOW` plus a second `fstat` of the opened descriptor, not the path | `open_regular` in `attachments/mod.rs` |

## 5. Filesystem default and URL absence

- `AttachmentPolicy::Disabled` is the default in the core (`filesystem_access_is_disabled_by_default`), the binding maps `allow_file_access.unwrap_or(false)`, and the facade sends the flag only when the caller sets it. A `path` attachment without the opt-in fails with `CONFIG_FILE_ACCESS_DISABLED` before any open (`rejects_file_attachments_without_opt_in`).
- There is no URL attachment type, field, or code path. `check-architecture` now fails on any `http://` or `https://` string in the core, the binding, or the shared TypeScript.

## 6. Caps before the network and before allocation

- `Delivery::send` validates the message and reads file attachments within the caps before the port is called; the port is a fake in tests, so "never reaches the port" means no connection was dialed.
- The facade copies every buffer attachment synchronously so the caller can reuse or mutate it immediately. That copy used to happen before any size check: an oversized payload was fully duplicated in the JS heap and only then rejected by the core.
- **Fixed in P11-003:** `ts/src/limits.ts` checks the declared `byteLength` of every buffer attachment, per file and then in aggregate, before the copy. `DEFAULT_LIMITS` mirrors `sendust_core::Limits`, and `check-architecture` now reads both sources and fails on drift. A lowered cap from `limits` is honoured.
- File attachments are not size-checked in the facade, because their size is unknown until read; the core enforces the same cap while reading.

## 7. Findings

| Severity | Finding | Decision |
| --- | --- | --- |
| Low | Facade copied buffer attachments before any cap check (fixed above) | Fixed: the pre-flight check runs before the copy and reports `LIMIT_ATTACHMENT_BYTES` / `LIMIT_ATTACHMENT_TOTAL` |
| Note | A buffer attachment is copied twice (facade, then the binding) | Accepted: the facade copy keeps the mutation contract, and the binding copy is the owned-memory boundary (ADR-0008). Both are bounded by the cap |
| Note | The caller's own buffer is still their allocation | Out of scope: the library cannot bound memory the caller already holds |

No high-severity finding. P11-005 re-runs this audit with the gate evidence.
