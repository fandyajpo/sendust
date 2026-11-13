# Architecture Amendment A01 — Multi-runtime and cross-platform support

**Status:** APPROVED — DESIGN AMENDMENT  
**Date:** 2026-11-13  
**Implementation status:** NOT STARTED. No native runtime/platform compatibility is claimed.

## Decision

A01 approves the multi-runtime product target and [ADR-0015](../architecture/decisions/0015-multi-runtime-support.md). It does not certify executable compatibility.

## Research evidence

The [multi-runtime research report](../research/multi-runtime-compatibility.md) reviews official Node.js, Bun, Deno, and NAPI-RS documentation:
- Node-API ABI stability does not prove alternate-runtime or OS/CPU/libc behavior.
- Bun implements Node-API but requires actual addon testing.
- Deno supports Node-API with local node_modules and --allow-ffi.
- NAPI-RS upstream treats Bun as best-effort and does not include Deno in blocking native-addon CI. Sendust must own blocking tests.

## Approved design constraints

- Exactly two Rust crates remain.
- Core remains free of Node-API and JavaScript runtime types.
- One Node-API 8 native implementation is the default.
- No speculative runtime adapters or WASM fallback.
- Node.js, Bun, and Deno are required MVP targets with blocking Sendust CI.
- Deno setup and FFI permission requirements are documented.
- Platform support is an explicit runtime × OS × CPU × ABI/libc matrix.
- Initial platform candidates: Linux x64 glibc, Linux ARM64 glibc, Linux x64 musl, macOS x64/ARM64, Windows x64 MSVC. Linux ARM64 musl is extended candidate.
- Only clean-install, load, and behavioral-tested combinations may be declared supported.

## Amendment work items

| ID | Work | Output | Blocks |
|---|---|---|---|
| A01-001 | Record official runtime compatibility evidence | Research report | A01-004 |
| A01-002 | Freeze multi-runtime/platform design | ADR-0015 and architecture contracts | A01-004 |
| A01-003 | Reconcile requirements, phases, CI, distribution and release docs | Traceability/consistency review | A01-004 |
| A01-004 | Amendment gate review | Design handoff; implementation tests remain blocking | P03 |

## Downstream gates

- **P03:** pinned three-runtime CI scaffold, runtime-neutral TypeScript boundary checks, platform metadata, no-postinstall policy.
- **P05:** native addon, Promise, buffer ownership, BUSY, close, GC/process shutdown, and error mapping on Node.js, Bun, and Deno.
- **P14:** build and clean-install each declared artifact; run fake-SMTP behavior on every claimed runtime/platform combination; publish exact support matrix.
- **P15/P16/P17:** document permissions, versions, platform requirements, artifacts, provenance, and final claims.

A runtime/platform combination failing a blocking test is unsupported and blocks its support claim. Reducing the required runtime set requires a superseding product decision and ADR.

## Explicit non-decisions

No additional Rust adapters, WASM, browser/edge/serverless support, Windows ARM64, FreeBSD, Android, Linux ARMv7, or extra CPU targets are approved. No queueing, pooling, retries, workers, or new SMTP transports.

## Gate result

**A01 design amendment: APPROVED.**  
**Runtime/platform implementation validation: NOT RUN / NOT APPROVED.**  
**Next execution phase: P03 Repository**, with amended requirements and downstream gates.
