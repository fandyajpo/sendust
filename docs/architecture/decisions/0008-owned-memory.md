# ADR-0008 — Copy JS buffers before async; bounded file reads

**Status:** ACCEPTED (planning)  
**Date:** 2026-11-13  
**Deciders:** Architecture review (P02-005), security review (P11-004)  
**Validated by:** P02-004, P05-002, P08-002, P11-004

## Context

Node-API values are scoped to a call and may be mutated, detached, or garbage collected. An async operation that retains a borrowed pointer into JS memory can read garbage, observe mutations, or crash. File attachments introduce a second ownership and trust boundary.

## Decision

- Every `Buffer`, `ArrayBuffer`, typed array, and string is copied into Rust-owned memory before the first `await`.
- No borrowed JS memory is stored in a future, transport state, or lettre message.
- Attachment files are read into bounded Rust-owned buffers only after explicit opt-in and regular-file validation.
- File reads use open-then-fstat semantics to reduce TOCTOU risk and never follow symlinks when the platform supports no-follow.
- Per-file, aggregate, and wire caps are enforced before bytes are written to the socket.

## Alternatives

1. **Zero-copy borrowed buffers.** Faster, but unsafe across async boundaries and vulnerable to mutation/detachment.
2. **Copy only large buffers.** Inconsistent ownership rules are harder to audit; rejected.
3. **Memory-map files.** Faster for large files, but complicates truncation/growth races and platform behavior; not justified under 10 MiB caps.

## Consequences

- Peak memory includes a copy of each input plus the serialized message.
- Caps keep the copy bounded.
- Mutation/detachment tests become mandatory.
- The ownership rule is simple enough to audit mechanically.

## Evidence required

- Buffer mutation and detachment tests.
- Ownership audit showing no borrowed pointer in async state.
- File-type matrix: regular, symlink, directory, FIFO, device, missing, unreadable.
- Cap boundary tests.
