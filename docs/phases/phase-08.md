# Phase 08 — Attachments

**Phase status:** IN PROGRESS  
**Phase dependency:** Phase 07, Phase 05  
**Companion references:** [system design](../architecture/system-design.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md)

## Objective

Implement optional, bounded regular-file attachments without URL support and with filesystem access disabled by default. Attachment bytes must be loaded safely, counted against hard caps, copied into Rust ownership, and rendered by Phase 07 MIME rules.

## Scope

In scope: attachment configuration, opt-in filesystem gate, regular-file validation, bounded reads, per-file and aggregate caps, MIME integration, and napi-facing descriptor conversion. Out of scope: URL or remote content, directories, symlink traversal, streaming from arbitrary devices, public TypeScript ergonomics beyond descriptors, and post-MVP resumable uploads.

## Tasks

### P08-001 — Define opt-in filesystem configuration and descriptor contract

- **Size:** M
- **Status:** IN REVIEW
- **Prerequisites:** P07-005, P05-005
- **Description:** Specify an explicit configuration that enables filesystem access; default is disabled. Define attachment descriptors with path, filename, and optional content type. Without opt-in, every path attachment fails before any filesystem operation. Propose a transport-scoped or send-scoped setting for approval in P02-004; do not assume it is frozen before the architecture gate. Never use ambient global state.
- **Evidence criteria:** Tests prove disabled-by-default rejects path attachments with no file open. Enabling access permits regular files only. Descriptor validation rejects missing fields, relative-path ambiguity, and unsupported content type forms with stable errors.

### P08-002 — Implement safe regular-file validation and bounded reads

- **Size:** L
- **Status:** IN REVIEW
- **Prerequisites:** P08-001
- **Description:** Open the target, verify it is a regular file via metadata, and read no more than the allowed bound. Reject symlinks, FIFOs, devices, directories, and sockets. Handle TOCTOU with an open-then-check approach and prevent unbounded growth. Read at most 10 MiB per attachment and stop immediately on over-limit. Do not follow URLs.
- **Evidence criteria:** Tests cover regular file, symlink, FIFO, device, directory, missing path, permission denied, and file that grows during read. All non-regular and over-limit cases fail without emitting partial message bytes. No network path exists.

### P08-003 — Enforce per-file, aggregate, and wire caps with attachments

- **Size:** M
- **Status:** IN REVIEW
- **Prerequisites:** P08-002, P04-003
- **Description:** Integrate attachment sizes into the 10 MiB per-file, 20 MiB aggregate raw attachment, and 30 MiB final wire caps. Define check ordering so per-file failure precedes aggregate and wire failure. Ensure MIME encoding overhead is included in wire size, not just raw bytes.
- **Evidence criteria:** Boundary tests cover 10 MiB and 10 MiB + 1; aggregate 20 MiB and 20 MiB + 1; final wire 30 MiB and 30 MiB + 1. Exact precedence is asserted. No SMTP connection is attempted after any cap failure.

### P08-004 — Integrate attachment bytes with MIME and copied ownership

- **Size:** L
- **Status:** IN REVIEW
- **Prerequisites:** P08-002, P08-003, P07-004
- **Description:** Pass loaded bytes into the MIME renderer as owned Rust data. Ensure no JS memory is borrowed across async I/O and no file handle or buffer is retained after send completion. Free memory promptly after serialization and enforce cleanup on cancellation/close.
- **Evidence criteria:** Tests mutate and detach source options and confirm sent bytes are unchanged. File descriptors are closed after success, validation failure, timeout, and close. Memory tests show attachment buffers are dropped after use.

### P08-005 — Attachment fault-matrix phase gate

- **Size:** M
- **Status:** IN REVIEW
- **Prerequisites:** P08-001, P08-002, P08-003, P08-004
- **Description:** Run all attachment security, boundary, and cleanup tests, including napi-path tests. Confirm filesystem access is disabled by default, no URL path exists, and all caps are hard. Review MIME output for filename safety and byte fidelity.
- **Evidence criteria:** Gate includes regular, missing, permission-denied, symlink, FIFO, device, directory, growing file, per-file over-limit, aggregate over-limit, wire over-limit, disabled opt-in, mutation/detachment, and post-close cleanup cases. Phase 09 may expose attachments only when this gate passes.

## Technical implementation details

Filesystem access is disabled by default and requires explicit opt-in configuration. Only bounded regular files may be read; symlinks, FIFOs, devices, directories, and sockets are rejected using open-then-verify semantics. No URL or remote fetch exists. Caps are hard: 10 MiB per attachment, 20 MiB aggregate raw attachment bytes, 30 MiB final wire message. Attachment bytes are Rust-owned before async work and never borrow JS memory. MIME metadata and rendering are owned by Phase 07. The Node-API layer copies descriptors and buffers per Phase 05. No pooling, retry, or SMTP behavior is added. Errors are stable and must not include file contents or credentials.

## Dependencies

Phase 08 requires Phase 07 and Phase 05 gates. P08-001 depends on both; P08-002 depends on P08-001; P08-003 adds P04-003; P08-004 adds P07-004; P08-005 depends on all. Phase 09 consumes P08-005.

## Risks and edge cases

TOCTOU, symlink races, special files, permission errors, files that grow during read, memory pressure from duplicated raw and encoded data, descriptor leaks on error, default-enabled filesystem access, and cap-precedence ambiguity are key risks. An attachment could be swapped between metadata check and read; open-then-fstat and bounded read from the opened handle are required.

## Testing requirements

Create local fixtures for regular, empty, exactly 10 MiB, 10 MiB + 1, multiple files totaling 20 MiB, 20 MiB + 1, and wire 30 MiB / 30 MiB + 1. Add symlink, FIFO, device when available, directory, missing, permission-denied, and growing-file fixtures. Add disabled-by-default, mutation/detachment, timeout-during-send, close-during-send, and repeated-success descriptor-leak tests. No external network or URL fixtures.

## Acceptance criteria

- Filesystem access is opt-in and disabled by default.
- Only bounded regular files are accepted; non-regular and URL inputs are impossible or rejected.
- All three caps are hard and deterministically ordered.
- Attachment bytes are owned, MIME-rendered, and cleaned up on every path.
- No JS memory borrowing, descriptor leak, or partial message on failure.

## Deliverables

Opt-in configuration, descriptor validation, safe file reader, cap enforcement, MIME integration, cleanup tests, and attachment gate report.

## Completion gate

`PHASE-08` is approved only when P08-005 proves the complete security/boundary matrix, default-disabled filesystem access, no URL path, and correct cleanup. Phase 09 may then expose attachment descriptors.

## Status

IN PROGRESS. P08-001 to P08-005 are IN REVIEW; the PHASE-08 gate record is submitted for project owner approval.

## Evidence log

Evidence labels follow the [AI implementation contract](../engineering/ai-implementation-contract.md).

| Task | Evidence | State |
| --- | --- | --- |
| P08-001 | `attachments/mod.rs`: `AttachmentPolicy` (`Disabled` by default, explicit `Enabled` opt-in, no ambient global), `filesystem_enabled`/`from_flag`, and descriptor path validation (absolute, non-empty, at most 1024 bytes, control-free). The policy is threaded through `Delivery`, `OutgoingMessage::validate`, `mime::serialize[_at]` and the Node-API adapter. Tests prove disabled-by-default with no open, relative/malformed path rejection, and opt-in acceptance of an absolute path. | IN REVIEW |
| P08-002 | `attachments/mod.rs`: `read_attachment` (re-exported from the facade) inspects the path with `symlink_metadata`, opens it with `O_NOFOLLOW`/`O_NONBLOCK` on unix, re-checks the opened descriptor with `fstat`, and reads at most the per-file cap plus one probe byte. Tests cover a regular file, the inclusive cap, cap + 1, an outgrowing file, a missing path, a directory, a symlink, a FIFO (no blocking), a character device, permission denied, and path-free errors. Non-regular and unreadable inputs fail with `CONFIG_ATTACHMENT_UNREADABLE`; over-cap input fails with `LIMIT_ATTACHMENT_BYTES`. | IN REVIEW |
| P08-003 | `attachments/mod.rs` checks every per-file cap before any aggregate check; `delivery/mod.rs` loads each file into owned `Attachment::Bytes` before composing, accumulating the raw total so the aggregate cap holds for file attachments too. Ordering is per-file, then aggregate, then wire; message validation runs before any file is opened, so identity and recipient errors keep their precedence, and every cap failure returns with zero port calls. Tests cover the inclusive per-file and aggregate limits, per-file precedence over aggregate, base64 overhead counted by the wire cap, and the no-dial guarantee. | IN REVIEW |
| P08-004 | `sendust-napi` accepts `AttachmentOptions` (path or `Uint8Array` content), copies buffer content into Rust-owned bytes, and rejects a shape with both or neither; `ts/src` adds the `Attachment` union, `Message.attachments`, and a synchronous snapshot in `toNativeAttachment` so a caller may reuse the array as soon as `send()` returns. `Delivery` reads file attachments inside the spawned async task, never on the Node thread, and hands owned bytes to the MIME renderer. Tests: buffer snapshot survives mutation after the call, a path attachment without opt-in fails `CONFIG_FILE_ACCESS_DISABLED`, an empty attachment fails `VALIDATION_INVALID_ATTACHMENT`, and the core file-loading tests run the descriptor through load and drop. | IN REVIEW |
| P08-005 | [QG-08 gate record](../gates/phase-08.md): the full attachment matrix (regular, missing, permission denied, symlink, FIFO, device, directory, growing file, per-file/aggregate/wire caps, disabled opt-in, buffer mutation and detach, descriptor release) across the core and the binding, the acceptance checklist, and the executed evidence. Submitted, not approved. | IN REVIEW |
