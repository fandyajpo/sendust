# ADR-0004 — napi-rs async exports on the napi-managed Tokio 1 runtime

**Status:** ACCEPTED (architecture baseline); Node-API 8 selected, runtime verification pending P05-003/P14-004  
**Date:** 2026-11-13  
**Deciders:** Architecture review (P02-005)  
**Source-reviewed by:** P01-004  
**Implementation gates:** P05-001, P05-003, P14-004

## Context

Sendust must expose asynchronous send and close operations to JavaScript without blocking the Node.js event loop. napi-rs v3 provides async exports that run on its managed Tokio runtime, converting the result to a JavaScript Promise. Source inspection of napi 3.16.2 shows its default features enable Node-API 4, not 8; the `napi8` feature must be enabled explicitly. lettre supports `Tokio1Executor`, but using the wrong runtime causes a panic.

Pin napi 3.16.2, napi-derive 3.7.0, and napi-build 2.6.0 for the initial implementation baseline. Enable napi's `napi8` and `async` features explicitly; napi 3.16.2 defaults to napi4 + dyn-symbols, while `napi8` raises the ABI floor and `async` enables the managed Tokio runtime. The selected Rust MSRV is at least 1.88 per the pinned napi source. Node-API 8 is the approved ABI floor; compilation and runtime compatibility remain hard tests at P05-003/P14-004.

## Decision

- Pin napi 3.16.2, napi-derive 3.7.0, and napi-build 2.6.0; enable napi8 and async explicitly; generate TypeScript declarations.
- Expose synchronous internal beginSend/beginClose preparation methods and async executeSend/waitClosed methods. Async work runs on the napi-managed Tokio 1 runtime. The TypeScript wrapper composes them into the public Promise-based send/close methods.
- Do not create, own, or shut down a separate Tokio runtime in the addon.
- Target Node-API 8; raise only through a superseding ADR if a future required feature cannot be implemented at this ABI level.
- Use `catch_unwind` at exported boundaries where the panic strategy permits; never build the addon with `panic = "abort"`.
- Keep all Node-API code inside `sendust-napi`; core stays Node-API-free but uses Tokio 1 for lettre networking and deadlines.

## Alternatives

1. **`AsyncTask` / `ThreadsafeFunction`.** More manual control, more boilerplate, and easier to make ownership mistakes; not needed for a simple request/response send.
2. **Synchronous export with an internal runtime.** Blocks the event loop; unacceptable.
3. **Custom runtime registration.** Adds complexity and a second lifecycle to validate; unnecessary unless WASI or non-Node targets are added.
4. **Node-API 4.** Broader ABI floor, but not selected for this design. Reconsider only through an ADR if runtime coverage requirements change.

## Consequences

- Promise-based API with correct rejection behavior.
- The binding must retain transport state across `await` using owned values, not borrowed JS memory.
- Blocking file reads must use a blocking pool, not the reactor.
- Forced GC must not close or corrupt a transport; close is explicit.
- Node-API level is part of the public support matrix and cannot change silently.

## Evidence required

- `napi` feature flags and Node-API level recorded in the manifest.
- Lifecycle tests on candidate Node 22/24/26: active send, forced GC, explicit close, process exit, close deadline.
- A test proving no second runtime is created.
- TypeScript declarations generated and type-checked.


## Amendment note — ADR-0015

ADR-0015 extends the runtime scope from Node.js-only to Node.js, Bun, and Deno while retaining Node-API 8 and napi-managed Tokio 1 as the shared design baseline. This ADR's Node-specific evidence remains required but is insufficient for alternate runtimes. Bun and Deno must pass Sendust-owned blocking async/lifecycle tests before support claims. No second Tokio runtime or speculative per-runtime Rust adapter is introduced.
