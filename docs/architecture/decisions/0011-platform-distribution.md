# ADR-0011 — Root package plus platform optional packages via napi-rs

**Status:** ACCEPTED (planning)  
**Date:** 2026-11-13  
**Deciders:** Architecture review (P02-005), release review (P16-005)  
**Validated by:** P14-002, P14-003, P14-004

## Context

Native addons must be distributed so consumers install a matching binary without a compiler or install-time download. napi-rs documents a model of a root package plus per-platform optional packages, selected by npm using `os`, `cpu`, and `libc` constraints.

## Decision

- Publish a root package that declares exact-version optional dependencies for each supported platform.
- Each platform package contains one `.node` artifact and declares `os`, `cpu`, and `libc` where applicable.
- The loader selects the matching package at runtime.
- No `postinstall` download from GitHub or a CDN.
- Missing or wrong-platform binaries produce a clear load error, never a silent fallback.
- The support matrix lists only targets built and runtime-tested in CI.

## Alternatives

1. **Single package with all binaries.** Large installs, npm platform constraints cannot select correctly, and irrelevant binaries ship everywhere.
2. **Source build on install.** Requires Rust toolchain for consumers; unacceptable.
3. **Postinstall binary download.** Introduces network, proxy, and integrity failure modes; napi-rs documents why this is avoided.
4. **WASM fallback.** Different runtime and performance characteristics; not the MVP path.

## Consequences

- Installation is fast and compiler-free.
- Publication is non-atomic and requires artifact-completeness gates and a recovery procedure.
- Version synchronization across root and platform packages is critical.
- Each target needs a build recipe and runtime validation.

## Evidence required

- `package.json` with exact optional dependencies and platform constraints.
- Artifact manifest and checksums.
- Clean-install tests on every declared target.
- Missing-binary and wrong-architecture negative tests.
- Post-publication verification record.
