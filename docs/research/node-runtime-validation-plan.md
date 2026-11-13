# Node-API and Runtime Validation Plan

**Status:** research/design complete; no native runtime probe executed  
**Research date:** 2026-11-13  
**Decision:** Pin napi 3.16.2, napi-derive 3.7.0, and napi-build 2.6.0. Explicitly enable napi8 and async on the napi crate; Node-API 8 and napi-rs managed Tokio 1 are the approved design baseline. Node 22/24/26 remain candidates, not support claims.

## 1. Source facts

- napi 3.16.2 defaults to napi4 + dyn-symbols. The napi8 feature explicitly raises the target ABI to Node-API 8. The async feature enables tokio_rt and the managed Tokio runtime.
- napi 3.16.2 source declares Rust MSRV 1.88. The exact addon toolchain and platform builds still require CI proof.
- Node-API ABI compatibility applies to later Node versions that implement the targeted Node-API level. It does not prove that a specific addon, dependency, OS, CPU, libc, or deployment target works.
- napi-rs v3's BufferSlice is documented as non-async/lifetime-bound; the owned Buffer/Vec path or an explicit synchronous copy must be used for data retained by async work.
- Async N-API argument references are rooted by napi-rs while work is pending, but that is not a substitute for Sendust's stronger ownership rule: all message data is copied into Rust-owned memory before async scheduling.
- The public TypeScript API must return Promise rejections for send-time validation/preparation errors, even though the internal native preparation step is synchronous.

Sources:
- [napi 3.16.2 Cargo features](https://github.com/napi-rs/napi-rs/blob/napi-v3.16.2/crates/napi/Cargo.toml)
- [napi-derive 3.7.0 Cargo manifest](https://github.com/napi-rs/napi-rs/blob/napi-derive-v3.7.0/crates/macro/Cargo.toml)
- [napi 3.16.2 Buffer implementation](https://github.com/napi-rs/napi-rs/blob/napi-v3.16.2/crates/napi/src/bindgen_runtime/js_values/buffer.rs)
- [napi-rs async functions](https://napi.rs/docs/concepts/async)
- [napi-rs support and compatibility](https://napi.rs/docs/introduction/compatibility)
- [napi 3.16.2 release metadata](https://docs.rs/crate/napi/3.16.2)

## 2. Candidate matrix

| Runtime | Build candidate | Runtime tested in this audit | Support claim |
| --- | --- | --- | --- |
| Node 22 | Yes; Node 22.13+ is within observed CLI compatibility range | No native addon test; audit shell has Node 22.16.0 but no Rust toolchain | Not supported/claimed yet |
| Node 24 | Yes | No | Not supported/claimed yet |
| Node 26 | Candidate only; validate release maturity at implementation time | No | Not supported/claimed yet |
| Bun / Deno | Not in MVP matrix | No | Unsupported / unclaimed |

Node-API 8 is the chosen ABI target. It must be confirmed against the exact napi-rs features at P05-001 and compiled/tested at P05-003/P14-004.

## 3. Exact native dependency configuration

```toml
napi = { version = "=3.16.2", features = ["napi8", "async"] }
napi-derive = "=3.7.0"
napi-build = "=2.6.0"
```

The default napi features include napi4 and dyn-symbols; enabling napi8 explicitly activates the Node-API 8 feature chain. The async feature enables tokio_rt. Do not use the full feature set because it enables newer Node-API levels beyond the approved floor.

## 4. Required binding shape

The public TypeScript wrapper exposes only createTransport, send, close, state, types, and stable errors. Internal native methods implement:

1. beginSend(message): synchronous conversion/validation; copy every Buffer, ArrayBuffer, typed array, and string into Rust-owned values; reserve the single-send slot; return an opaque owned operation.
2. executeSend(operation): asynchronous execution on napi-managed Tokio 1; returns a Promise.
3. beginClose(): synchronous open→closing transition and cancellation signal.
4. waitClosed(): Promise that awaits the active send and bounded cleanup.

The TypeScript send() wrapper composes beginSend and executeSend. If beginSend fails synchronously, send() returns Promise.reject(mappedError), so the public Promise contract remains consistent. close() composes beginClose and waitClosed.

No second Tokio runtime may be constructed. The Rust core has no Node/N-API dependency.

## 5. Runtime probe matrix for P05-003/P14-004

| Probe | Setup | Expected result |
| --- | --- | --- |
| Async runtime | Run a fake SMTP transaction through napi-managed Tokio | No missing-runtime panic; Promise settles |
| Buffer mutation | Call send, mutate original Buffer immediately after return | SMTP bytes match the pre-call snapshot |
| ArrayBuffer detachment | Transfer/detach original backing store immediately after return | Send uses owned snapshot; no crash/corruption |
| BUSY | Invoke two sends in the same JS turn | Second returns BUSY without opening a second connection |
| Close during connect | Hold fake relay greeting; call close | No new sends; active operation cancels within close budget |
| Close during DATA | Hold fake relay after DATA; call close | If final disposition is not observed after message transmission begins, send resolves unknown |
| Double close | Call close concurrently and repeatedly | Same bounded close completion; no double free |
| Forced GC | Drop JS references and force GC during send | In-flight operation remains valid; GC does not close transport |
| No explicit close | Drop transport and allow process shutdown | No finalizer-dependent correctness; no permanent event-loop hang |
| Process exit | Terminate child process while send is active | No unbounded shutdown; socket/task cleanup is bounded |
| Candidate matrix | Run all tests on Node 22, 24, 26 where available | Only passing, blocking-CI targets may be claimed supported |

## 6. Acceptance rule

A runtime is supported only after the native addon builds and all blocking tests pass on that exact Node version and platform. Node-API compatibility alone is insufficient. Until then, Node 22/24/26 are candidates only.

## 7. Multi-runtime amendment

The original Node-only scope is extended by [multi-runtime compatibility research](multi-runtime-compatibility.md) and ADR-0015. Node.js, Bun, and Deno are required MVP targets. Node-specific probes remain necessary but are no longer sufficient.

P05/P14 MUST run the full Sendust behavioral suite on exact pinned Node.js, Bun, and Deno versions. Deno additionally requires a clean consumer using local node_modules and --allow-ffi. Bun and Deno support MUST NOT be inferred from Node-API ABI compatibility or NAPI-RS upstream CI.

No Sendust native addon was executed on Bun or Deno during research. Runtime support claims remain unvalidated until blocking Sendust CI passes.
