# ADR-0015 — Multi-runtime Node-API support and server platform policy

**Status:** ACCEPTED (design amendment; runtime/platform evidence remains release-gated)  
**Date:** 2026-11-13  
**Amends:** ADR-0004 runtime scope and ADR-0011 distribution/support claims  
**Research:** [Multi-runtime compatibility research](../../research/multi-runtime-compatibility.md)  
**Approval:** Architecture Amendment A01

## Context

The original architecture targeted Node.js only. Product intent now explicitly includes Node.js, Bun, and Deno, plus major server OS/CPU targets. Node-API provides a shared ABI surface, but alternate runtimes have independent implementations, loaders, permissions, and compatibility gaps. NAPI-RS upstream treats Bun as best-effort and Deno is outside its blocking native-addon CI.

## Decision

1. Node.js, Bun, and Deno are required MVP runtime targets. Each is blocking, not best-effort. No runtime is declared supported until Sendust's own native addon, async, lifecycle, SMTP, and clean-install suite passes on exact versions.
2. Retain exactly two Rust crates. `sendust-core` remains independent of JavaScript runtimes and Node-API. `sendust-napi` remains the sole native binding implementation using Node-API 8.
3. Do not add separate Bun/Deno Rust adapters, extra crates, or WASM fallback unless executable compatibility evidence requires them and a superseding ADR approves.
4. Public TypeScript API remains runtime-neutral. Isolate native loading in the loader and avoid Node-only globals/built-ins in shared code.
5. Deno usage must document and test local `node_modules` and explicit `--allow-ffi`. No npm postinstall compilation, install-time build, or binary download.
6. Support is the intersection of exact tested runtime version, OS, CPU, and native ABI/libc target. Compile-only targets are not supported.
7. Initial platform candidates: Linux x64 glibc, Linux ARM64 glibc, Linux x64 musl, macOS x64, macOS ARM64, Windows x64 MSVC. Linux ARM64 musl is an extended candidate. Additional platforms are not implied.
8. Pin exact runtime versions in CI. Node.js 22/24/26 remain candidates until Sendust tests pass; pin stable Bun and Deno versions at implementation freeze. Publish only exact tested versions/ranges.
9. Blocking tests cover addon loading, Promise/async behavior, SMTP outcomes, buffer ownership, BUSY, close races, shutdown, TLS, errors, and clean install. All three runtimes must pass before MVP runtime approval.
10. This decision adds no queue, worker, retry, pooling, background delivery, extra SMTP transport, browser/edge, or WASM support.

## Alternatives

- Node.js only: rejected because it contradicts the explicit product target.
- Separate Rust binding per runtime: rejected as premature duplication.
- WASM universal layer: rejected for MVP because it adds a separate networking, TLS, permission, and distribution surface.
- Treat Node-API ABI as universal proof: rejected because ABI stability does not prove alternate-runtime behavior, loader, or platform ABI compatibility.

## Consequences

P03 establishes multi-runtime CI and loader guardrails. P05 validates native binding/lifecycle on Node.js, Bun, and Deno. P14 validates runtime × OS × CPU × ABI distribution and clean installs. Deno docs include permission and package-layout requirements. Some combinations may be excluded if they cannot be tested reliably.

## Evidence required

- Official-source research report.
- Blocking CI for all three runtimes on every claimed platform, or an explicitly approved reduced matrix.
- Clean consumer install and native load for every claimed combination.
- Full fake-SMTP, ownership, lifecycle, TLS, and error contract tests.
- Release support matrix with exact versions, CI evidence, and known limitations.

This ADR approves the design target only. It does not claim Bun, Deno, or any new platform has passed Sendust runtime tests.
