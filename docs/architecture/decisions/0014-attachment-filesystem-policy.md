# ADR-0014 — Attachment filesystem policy

**Status:** ACCEPTED for planning; implementation evidence required at P08-005 and P11-003.  
**Date:** 2026-11-13.

## Context

Allowing callers to pass arbitrary filesystem paths creates read and denial-of-service risks: symlinks can escape intended directories, FIFOs and devices can block, directories can be unbounded, and files can change between validation and read. Allowing URLs introduces network access and SSRF. The MVP needs attachments but does not need remote or streamed sources.

## Decision

- Filesystem attachment access is **disabled by default** and must be explicitly enabled in transport configuration.
- When enabled, Sendust opens the path with no-follow semantics where the platform supports it, then calls `fstat` on the opened descriptor and requires a regular file.
- Symlinks, directories, FIFOs, devices, sockets, and unreadable files are rejected with stable errors.
- Reads are bounded by the per-file cap plus one probe byte; overflow is rejected.
- Absolute paths are never echoed in public errors.
- Buffer attachments are always supported within caps.
- URL, stream, glob, directory, and remote attachment forms are not part of the API.

## Alternatives

1. **Allow any path by default.** Convenient, but unsafe and surprising.
2. **Resolve and allow symlinks.** Weakens the trust boundary and complicates TOCTOU reasoning.
3. **Allow URL attachments.** Adds SSRF, redirect, timeout, and network trust surface; explicitly out of scope.
4. **Read the whole file without a cap probe.** Allows unbounded memory growth and race-based overflow.

## Consequences

- Callers must opt in explicitly, which is documented.
- Attachment behavior is testable with a deterministic file-type matrix.
- Some legitimate large-file or remote-source use cases are deferred to post-MVP streaming.
- Error messages use caller-supplied filenames or a redacted marker, not absolute paths.

## Evidence required

- Default-denial tests.
- File-type matrix: regular, symlink, directory, FIFO, device, missing, unreadable.
- Cap boundary tests at limit and limit + 1.
- Redaction scans in P11-004.
