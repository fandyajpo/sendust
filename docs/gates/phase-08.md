# QG-08 — Attachments Phase Record

**Decision:** SUBMITTED — awaiting project owner approval  
**Phase:** P08 — Attachments  
**Gate:** PHASE-08  
**Prepared:** 2026-11-15  
**Branch:** `main`  
**Evidence:** the local Rust, checker, and binding commands below, plus the CI run for this record's commit.

This record is the review package for PHASE-08. The implementation was written by
an agent, so the approval in section 6 is the required human review step, not a
self-approval. P08-001 to P08-005 remain `IN REVIEW` until that approval.

## 1. Task evidence

| Task | Status | Evidence |
| --- | --- | --- |
| P08-001 | IN REVIEW | `attachments/mod.rs`: `AttachmentPolicy` (`Disabled` by default, explicit `Enabled`, no ambient global), `filesystem_enabled`/`from_flag`, descriptor validation (absolute, non-empty, ≤1024 bytes, control-free), threaded through `Delivery`, `OutgoingMessage::validate`, `mime::serialize[_at]` and the binding |
| P08-002 | IN REVIEW | `attachments/mod.rs`: `read_attachment` checks the path type with `symlink_metadata`, opens with `O_NOFOLLOW`/`O_NONBLOCK` on unix, re-checks the opened descriptor with `fstat`, and reads at most the per-file cap plus one probe byte; `CONFIG_ATTACHMENT_UNREADABLE` for non-regular or unreadable input, `LIMIT_ATTACHMENT_BYTES` for overflow |
| P08-003 | IN REVIEW | Per-file checks run before aggregate ones; `Delivery` loads files into owned `Attachment::Bytes` before composing and accumulates the raw total, so the aggregate cap covers file attachments. Ordering is per-file → aggregate → wire, and message validation runs before any open so identity/recipient errors keep precedence |
| P08-004 | IN REVIEW | `sendust-napi` accepts `AttachmentOptions` (path or `Uint8Array`), copies buffer content into Rust-owned bytes, rejects a shape with both or neither; `ts/src` adds the `Attachment` union, `Message.attachments`, and a synchronous snapshot so a caller may reuse the array as soon as `send()` returns. File reads happen inside the spawned async task, never on the Node thread |
| P08-005 | IN REVIEW | This record, the matrix in section 3, and the executed evidence in section 5 |

## 2. Acceptance checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Filesystem access is opt-in and disabled by default | PASS | `filesystem_access_is_disabled_by_default`, `rejects_file_attachments_without_opt_in`, binding test `rejects a path attachment without filesystem opt-in` |
| Only bounded regular files are accepted; non-regular and URL inputs are impossible or rejected | PASS | `rejects_a_symlink_to_a_regular_file`, `rejects_a_directory`, `rejects_a_fifo_without_blocking`, `rejects_a_character_device`, `rejects_a_missing_path`, `reports_an_unreadable_file_as_unreadable`; no URL/remote attachment type exists in the model, binding, or facade |
| All three caps are hard and deterministically ordered | PASS | `enforces_the_per_file_cap_inclusively`, `enforces_the_aggregate_cap`, `a_per_file_violation_precedes_an_aggregate_one`, `enforces the per-file cap for a path attachment`, `enforces the aggregate cap across path attachments`, `the_wire_cap_counts_attachment_encoding` |
| Attachment bytes are owned, MIME-rendered, and cleaned up on every path | PASS | `file_attachments_are_loaded_into_the_wire` (filename and base64 body in the wire), `the_file_handle_is_closed_after_a_read`, `errors_never_carry_the_path`, `file_attachments_are_loaded_into_the_wire` in the binding via `copies a buffer attachment before the send returns` |
| No JS memory borrowing, descriptor leak, or partial message on failure | PASS | The binding copies `Uint8Array` content into owned bytes and the facade snapshots before returning; `an_oversized_file_never_reaches_the_port`, `an_aggregate_over_the_limit_never_reaches_the_port`, `the_wire_cap_counts_attachment_encoding` and `a_message_error_precedes_a_file_read_error` all assert zero port calls |

## 3. Attachment fault matrix

| Case | Test | Result |
| --- | --- | --- |
| Regular file within the cap | `attachments::file_reads::reads_a_regular_file` | PASS |
| Regular file exactly at the per-file cap | `attachments::file_reads::enforces_the_per_file_cap_inclusively` | PASS |
| Regular file at per-file cap + 1 | `attachments::file_reads::enforces_the_per_file_cap_inclusively`, `delivery::tests::an_oversized_file_never_reaches_the_port` | PASS — `LIMIT_ATTACHMENT_BYTES`, no dial |
| File that grows past the cap | `attachments::file_reads::rejects_a_file_that_outgrows_the_cap` | PASS — the one-byte probe catches it |
| Missing path | `attachments::file_reads::rejects_a_missing_path` | PASS — `CONFIG_ATTACHMENT_UNREADABLE` |
| Directory | `attachments::file_reads::rejects_a_directory` | PASS |
| Symlink to a regular file | `attachments::file_reads::rejects_a_symlink_to_a_regular_file` | PASS |
| FIFO (must not block) | `attachments::file_reads::rejects_a_fifo_without_blocking` | PASS |
| Character device | `attachments::file_reads::rejects_a_character_device` | PASS (`/dev/null`) |
| Permission denied | `attachments::file_reads::reports_an_unreadable_file_as_unreadable` | PASS (skips under root, where the mode is ignored) |
| Aggregate at 20 MiB + 1 | `attachments::tests::enforces_the_aggregate_cap`, `delivery::tests::an_aggregate_over_the_limit_never_reaches_the_port`, binding `enforces the aggregate cap across path attachments` | PASS — `LIMIT_ATTACHMENT_TOTAL` |
| Wire cap counts base64 overhead | `delivery::tests::the_wire_cap_counts_attachment_encoding` | PASS — 1 KiB raw against a 1 KiB wire cap is `LIMIT_WIRE_BYTES` |
| Filesystem disabled by default | `attachments::tests::rejects_file_attachments_without_opt_in`, binding `rejects a path attachment without filesystem opt-in` | PASS — `CONFIG_FILE_ACCESS_DISABLED`, no open |
| Buffer mutation after `send()` | binding `copies a buffer attachment before the send returns` | PASS — the wire still reflects the original bytes |
| Buffer detach after `send()` | binding `snapshots a buffer before the caller detaches it` | PASS — the send continues with the snapshot |
| File handle released after a read | `attachments::file_reads::the_file_handle_is_closed_after_a_read` | PASS — the file can be removed immediately afterwards |
| Invalid attachment shape | binding `rejects an attachment with neither a path nor content` | PASS — `VALIDATION_INVALID_ATTACHMENT` |
| Error text never carries a path | `attachments::file_reads::errors_never_carry_the_path` | PASS |

## 4. Security review

- Filesystem access is off unless the transport explicitly enables it; the policy
  is a value, never ambient state.
- The path is type-checked before any open, and the open itself refuses a symlink
  (`O_NOFOLLOW`) and cannot block on a FIFO (`O_NONBLOCK`); the descriptor is then
  `fstat`ed and must be a regular file, so a path swap cannot change what is read.
- Reads are bounded by the per-file cap plus one probe byte, so a growing file is
  rejected instead of partially read.
- No URL, HTTP, stream, directory, or glob attachment form exists in the model, the
  binding, or the facade.
- Absolute paths never appear in an error message or in `details`; a test scans the
  rendered error for the fixture path.
- The aggregate cap is accumulated while files are read, so many small files cannot
  exceed 20 MiB of raw bytes before the check runs.
- No credential, message body, or attachment bytes are logged or serialized into an
  error.

## 5. Executed evidence

Run on macOS with the pinned toolchain on 2026-11-15:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
node scripts/check-architecture.mjs
node scripts/check-docs.mjs
node scripts/check-secrets.mjs
npm run build:native
npm --workspace ./ts run lint
npm --workspace ./ts run format:check
npm --workspace ./ts run typecheck
npm --workspace ./ts run test
node tests/runtime/smoke.mjs
```

Result: 164 core unit tests (24 of them in `attachments`), 9 harness, 6 client, 4
MIME conformance, 6 MIME body fixtures, 20 SMTP fault, 6 session, and 13 binding
tests pass; every checker passes; the Node runtime smoke reports 5 binding checks.

CI on `main` at `6ffe2dd` passed all eight jobs: Rust workspace, Native binding
(Node.js 22), TypeScript on Node.js 22.23.5/24.22.3/26.13.1, Bun, Deno, and the
architecture-and-documentation job.

## 6. Decision

**SUBMITTED, not approved.** The project owner must review this record and its
evidence. On approval, P08-001 to P08-005 move to `COMPLETED`, PHASE-08 is
recorded as approved with the date, and P09 becomes eligible. Until then the tasks
stay `IN REVIEW` and Phase 09 must not expose attachments.

Approval would cover P08 only. A failed compile, test, checker, or runtime result
on `main` reopens this gate.

## 7. Deferred (recorded, not claimed)

- The core `mime::serialize` entry point still refuses `Attachment::File`; callers
  go through `Delivery`, which loads the bytes first. This is deliberate: the
  serializer stays pure and I/O-free.
- File reads run on a Tokio worker thread inside the spawned send task, not on the
  Node event loop. Moving them to `spawn_blocking` is a P10 reliability concern.
- Windows has no `O_NOFOLLOW` equivalent in the standard library; it relies on the
  type check plus the descriptor re-check.
- Descriptor-leak testing under repeated sends and post-close cleanup belong to the
  P11-003 security audit, which broadens this matrix.
