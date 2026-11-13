# ADR-0004 — napi-rs async exports on the napi-managed Tokio 1 runtime

**Status:** ACCEPTED (planning); Node-API level pending P01-004  
**Date:** 2026-11-13  
**Deciders:** Architecture review (P02-005)  
**Validated by:** P01-004, P05-001, P05-003

## Context

Sendust must expose asynchronous send and close operations to JavaScript without blocking the Node.js event loop. napi-rs v3 provides `async fn` exports that run on a Tokio runtime it manages, converting the result to a JavaScript Promise. lettre supports `Tokio1Executor`, but using the wrong runtime causes a panic.

The scaffold defaults to Node-API 4; Sendust proposes Node-API 8 for a wider feature floor while retaining broad ABI compatibility. The exact floor required by the selected napi features must be verified.

## Decision

- Use napi-rs v3 with generated TypeScript declarations.
- Expose async `send` and `close` via `#[napi] async fn`, running on the napi-managed Tokio 1 runtime.
- Do not create, own, or shut down a separate Tokio runtime in the addon.
- Target Node-API 8 initially; raise only if P01-004 proves a required feature needs more.
- Use `catch_unwind` at exported boundaries where the panic strategy permits; never build the addon with `panic = "abort"`.
- Keep all Node-API code inside `sendust-napi`; core stays runtime-agnostic.

## Alternatives

1. **`AsyncTask` / `ThreadsafeFunction`.** More manual control, more boilerplate, and easier to make ownership mistakes; not needed for a simple request/response send.
2. **Synchronous export with an internal runtime.** Blocks the event loop; unacceptable.
3. **Custom runtime registration.** Adds complexity and a second lifecycle to validate; unnecessary unless WASI or non-Node targets are added.
4. **Node-API 4.** Broader floor, but may lack features needed for safe async/error handling. Validate before choosing.

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
