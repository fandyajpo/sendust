# Multi-Runtime and Cross-Platform Compatibility Research

**Status:** source research completed; Sendust-native validation pending  
**Research date:** 2026-11-13

## Executive finding

Node-API is the most economical shared native binding path for Node.js, Bun, and Deno. It is not a universal runtime or platform guarantee. Sendust must test its actual addon, async execution, loader, lifecycle, package installation, and SMTP behavior on each claimed runtime/platform combination.

Keep exactly two Rust crates: runtime-independent `sendust-core` and thin `sendust-napi`. Keep one native implementation as the default. Do not add separate Bun/Deno adapters or WASM unless a reproducible compatibility failure proves they are necessary.

## Official source findings

### Node.js
Node-API is independent of V8 and ABI-stable across Node.js releases implementing the selected API level. Node-API 8 is available in Node.js 12.22.0/14.17.0/15.12.0/16.0.0 and later compatible releases. This does not guarantee OS/CPU/libc compatibility, alternate runtime behavior, or compatibility of native dependencies.

Source: https://nodejs.org/api/n-api.html

### Bun
Bun implements Node-API and documents loading `.node` addons through `require()` or `process.dlopen`. Bun also notes that not all native addons necessarily work; actual load-and-function tests are required on the target OS/CPU.

Sources:
- https://bun.sh/docs/runtime/node-api
- https://bun.sh/docs/runtime/nodejs-compat

### Deno
Deno supports Node-API addons, but documents a local `node_modules` requirement and explicit `--allow-ffi` permission. Deno does not run arbitrary npm lifecycle scripts by default, so Sendust must not rely on postinstall compilation or binary downloads.

Sources:
- https://docs.deno.com/runtime/fundamentals/node/
- https://docs.deno.com/runtime/reference/permissions/
- https://docs.deno.com/runtime/fundamentals/ffi/

### NAPI-RS
Current NAPI-RS compatibility guidance distinguishes ABI compatibility, build CLI support, target triples, and actual CI coverage. It identifies Node.js as primary, Bun as best-effort in upstream CI, and Deno as outside its blocking native-addon CI matrix. Its current source CI tests Node.js 22/24/26; Bun is non-blocking. Sendust must own blocking Bun and Deno tests.

Source: https://napi.rs/docs/more/support-compatibility

## Compatibility dimensions

1. Runtime Node-API implementation compatibility.
2. Native addon loading and loader resolution.
3. Async Promise settlement and Tokio execution.
4. Lifecycle: BUSY, close races, GC, process shutdown.
5. Package-manager behavior and optional platform dependency selection.
6. OS/CPU/libc ABI and native system dependencies.
7. Clean deployment and SMTP/TLS behavior.

Passing one dimension does not imply the others pass.

## Required MVP runtime policy

| Runtime | MVP intent | Required evidence | Special handling |
|---|---|---|---|
| Node.js | Required | Blocking Sendust native-load, lifecycle, and fake-SMTP CI | Pin exact tested versions from the toolchain baseline |
| Bun | Required | Same blocking suite on pinned stable Bun | Verify actual Node-API and loader behavior |
| Deno | Required | Same blocking suite on pinned stable Deno | Local node_modules and --allow-ffi |

Exact Node.js, Bun, and Deno CI candidates are pinned in [the toolchain baseline](../engineering/toolchain-baseline.md). They remain candidates until Sendust tests pass. Publish only exact tested versions/ranges. If any runtime fails a blocking test, it remains unsupported and blocks the MVP runtime gate unless a superseding product decision revises the target.

## Proposed MVP server platform baseline

| OS | CPU | ABI | Status |
|---|---|---|---|
| Linux | x64 | glibc | Required candidate |
| Linux | ARM64 | glibc | Required candidate |
| Linux | x64 | musl | Required candidate |
| macOS | x64 | Darwin | Required candidate |
| macOS | ARM64 | Darwin | Required candidate |
| Windows | x64 | MSVC | Required candidate |
| Linux | ARM64 | musl | Extended candidate |

Every claimed runtime/platform combination must pass clean install, native load, and behavioral tests. Compile-only targets are not supported. Future candidates, not MVP promises: Windows ARM64, FreeBSD, Linux ARMv7, Android, other Linux architectures, serverless/edge, browser, and WASI.

## Loader and TypeScript boundary

- Keep the public API runtime-neutral.
- Isolate native loading in `ts/src/loader.ts`.
- Avoid assuming Node-only globals or built-ins in shared wrapper code.
- Audit the generated NAPI-RS loader on all three runtimes.
- If needed, use the smallest runtime-specific loader branch while retaining one native implementation and identical public API.
- Deno instructions must explain local node_modules and --allow-ffi.
- No postinstall compilation, runtime source compilation, or network binary downloads.
- Separate Rust adapters require a failing compatibility test and superseding ADR.

## Blocking test suite

On every claimed runtime/platform combination:
- native addon import and correct binary selection;
- createTransport and invalid configuration;
- accepted fake-SMTP send;
- mixed RCPT, final DATA rejection, and unknown-after-DATA;
- Promise settlement and stable error codes;
- Buffer/ArrayBuffer/typed-array snapshot before async;
- BUSY and explicit close;
- close during connect/DATA, double close, send after close;
- process shutdown;
- TLS verification fixtures;
- clean installation without real relay, external network, credentials, install-time compiler, or postinstall download.

Record exact runtime version, OS, CPU, libc/minimum OS, artifact checksum, and CI run.

## Open implementation validations

1. Does napi-rs 3.16.2 async execution work under Bun and Deno with Sendust's Tokio/lettre future?
2. Does the generated loader resolve platform packages in all three runtimes?
3. Does Deno load the exact package layout with local node_modules and --allow-ffi?
4. Does close/process shutdown behave consistently?
5. Can every required platform artifact be built, loaded, and tested?

No Sendust native addon was built or executed during this research. This is source research, not implementation validation.
