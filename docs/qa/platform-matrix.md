# Cross-Platform Build and Artifact Matrix (P14-003)

**Status:** specified — no target is built or supported yet
**Date:** 2026-11-16
**Owner:** P14-004 for validation; the project owner for publication layout

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
| macOS | x64 | Darwin | none available | `x86_64-apple-darwin` | `sendust-macos-x64.node` | NOT BUILT — no Intel runner in this repository |
| Linux | x64 | glibc | `ubuntu-24.04` | `x86_64-unknown-linux-gnu` | `sendust-linux-x64-gnu.node` | CI BUILD + LOAD: PASS |
| Linux | arm64 | glibc | `ubuntu-24.04-arm` | `aarch64-unknown-linux-gnu` | `sendust-linux-arm64-gnu.node` | CI BUILD + LOAD: PASS |
| Windows | x64 | MSVC | `windows-2022` | `x86_64-pc-windows-msvc` | `sendust-windows-x64.node` | CI BUILD + LOAD: PASS |

Every row builds with the pinned Rust 1.100.0 toolchain on a native runner via `npm run build:artifact`, which compiles `sendust-napi` and stages `sendust-<os>-<arch>[-<libc>].node` under `artifacts/`. The same runner then loads the artifact to prove it is the target it claims to be — no cross-compiled, unloaded binary counts. The feature set is identical everywhere: Node-API 8 (`napi8`, `async`), `rustls` with the `ring` provider and `webpki-roots`, no `native-tls`, no alternate verifier. The architecture check enforces those rules for every build, so no target can weaken TLS.

The CI job `Native artifact (<target>)` runs the build and the load check on each runner and uploads the artifact. The four rows marked PASS were observed on `373dda6`…`5467c3a`; `npm run verify:platform` additionally installs the facade plus the platform package in a clean consumer and loads it through `@sendust/ts`. macOS x64 has no Intel runner in this repository — its job stayed queued and was removed, so that row stays NOT BUILT rather than being advertised.

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

## 5. Validation owner and next step

P14-004 owns clean install, load, binary selection, accepted send, BUSY, close and the missing/wrong-binary negatives **per row** once the publication layout is decided. Until then this document is a specification only: nothing here is a support claim (C-06, C-35), and the current package remains macOS arm64 from the local build.

Open decision for the project owner: choose layout 1 or 2 above. Both keep the single addon per platform and the no-postinstall rule; they differ only in how artifacts are distributed.
