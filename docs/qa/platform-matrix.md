# Cross-Platform Build and Artifact Matrix (P14-003)

**Status:** all five targets built and validated (P14-004)
**Date:** 2026-11-17
**Owner:** P14-004 for validation; P14-005 for the gate; the project owner for publication

## 1. Publication layout

**Decided (ADR-0020): per-platform packages.**

- `@sendust/ts` stays the facade and bundles a `sendust.node` only for local builds.
- Each target publishes `@sendust/ts-<platform>-<arch>[-<libc>]` holding just its addon: `linux-x64-gnu`, `linux-arm64-gnu`, `macos-x64`, `macos-arm64`, `windows-x64`.
- The facade declares them as `optionalDependencies`; npm skips the ones that do not match the platform.
- The loader tries the bundled addon first (source tree, local build), then the mapped optional package, then fails with `CONFIG_NATIVE_UNAVAILABLE` naming the expected package. Unsupported platforms get the same stable code with a "no native build" message.

Until a target is validated by P14-004 and published, the platform has no binary and no support claim.

## 2. Candidate matrix

Artifact naming: `sendust-<os>-<arch>[-<libc>].node`

| OS | Arch | Libc/ABI | Runner | Build target | Artifact | Status |
| --- | --- | --- | --- | --- | --- | --- |
| macOS | arm64 | Darwin | `macos-15` | `aarch64-apple-darwin` | `sendust-macos-arm64.node` | CI BUILD + LOAD: PASS |
| macOS | x64 | Darwin | `macos-15-intel` | `x86_64-apple-darwin` | `sendust-macos-x64.node` | CI BUILD + LOAD: PASS |
| Linux | x64 | glibc | `ubuntu-24.04` | `x86_64-unknown-linux-gnu` | `sendust-linux-x64-gnu.node` | CI BUILD + LOAD: PASS |
| Linux | arm64 | glibc | `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` | `sendust-linux-arm64-gnu.node` | CI BUILD + LOAD: PASS |
| Windows | x64 | MSVC | `windows-2022` | `x86_64-pc-windows-msvc` | `sendust-windows-x64.node` | CI BUILD + LOAD: PASS |

Every row builds with the pinned Rust 1.100.0 toolchain on a native runner via `npm run build:artifact`, which compiles `sendust-napi` and stages `sendust-<os>-<arch>[-<libc>].node` under `artifacts/`. The same runner then loads the artifact to prove it is the target it claims to be — no cross-compiled, unloaded binary counts. The feature set is identical everywhere: Node-API 8 (`napi8`, `async`), `rustls` with the `ring` provider and `webpki-roots`, no `native-tls`, no alternate verifier. The architecture check enforces those rules for every build, so no target can weaken TLS.

The CI job `Native artifact (<target>)` builds and loads the artifact on each runner, runs the facade behaviour suite against that artifact as the bundled addon, and installs the facade plus the platform package in a clean consumer through `npm run verify:platform`. Every row marked PASS was observed on `373dda6`…`c0005e0`, including the missing-binary and corrupt-binary negatives; the details are in [platform validation](platform-validation.md). macOS x64 runs on the `macos-15-intel` runner, so it is built and loaded natively rather than cross-compiled.

## 3. Build recipe

```text
rustup target add <target>            # where the runner is not the host
cargo build -p sendust-napi --release [--target <target>]
# the cdylib is target/[<target>/]release/libsendust_napi.{so,dylib} on Unix
# and target/[<target>/]release/sendust_napi.dll on Windows
# renamed to the artifact name in section 2 and uploaded
```

The artifact is renamed at the end of the build; no artifact may be produced by a compile-only step that has not run the node-API load check on the same runner.

## 4. Explicitly unsupported in the MVP

These must not appear in publication inputs:

- Linux arm64 musl and x64 musl — needs cross tooling and a musl runtime check that is not set up.
- Windows arm64 MSVC — no runner or support claim.
- Big-endian targets — untested and not claimed (R-06).
- WASM, serverless, edge and browser environments — outside the product (C-36, C-30).

No “best effort” build may be published: a target is either in the matrix above with passing P14-004 evidence, or absent.

## 5. Validation result and next step

P14-004 validated every row against its own artifact at Node.js 22.23.5: clean-consumer install, load and binary selection, accepted send, BUSY, close, and the missing/wrong-binary negatives. The five PASS rows therefore carry support claims for Node.js on those exact OS/arch pairs; results and method are in [platform validation](platform-validation.md).

Still open, and owned outside this task:

- Bun and Deno install the packed facade in a fresh consumer and run it, plus a binding smoke, a facade smoke, and a behaviour harness per target; the full Vitest suite stays on Node.js, so that breadth is Node.js evidence.
- The published version ranges for the platform packages are only exercised at publication (P17); `prepare:publish` adds them and a local selection test proves the install choice.

P14-005 consolidates this matrix with the QA and performance evidence for the phase gate; no row may be advertised beyond what it records.
