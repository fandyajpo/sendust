# QG-05 — Node-API Binding Phase Record

**Decision:** APPROVED — project owner review, 2026-11-14  
**Phase:** P05 — Binding  
**Gate:** PHASE-05  
**Prepared:** 2026-11-14  
**Branch:** `test/runtime-smoke` (merged as `0d806bc`)  
**Evidence:** 16/16 CI checks on the merged commit and on its branch head; local Rust, binding, runtime, and live-relay evidence recorded below.

## 1. Task evidence

| Task | Status | Evidence |
| --- | --- | --- |
| P05-001 | COMPLETED | `crates/sendust-napi/src/lib.rs`: `Transport` with `constructor`, `state`, `send`, `close`, `nativeVersion`; object types for options, message, recipients, response, result. No SMTP, MIME, validation, or delivery policy in the binding |
| P05-002 | COMPLETED | Every JavaScript value is converted to owned Rust values (`String`, `Vec<String>`, `u32`) before the first `await`; no borrowed memory is retained and no Buffer is accepted yet (attachments are P08) |
| P05-003 | COMPLETED | One `tokio::sync::Mutex` slot plus an atomic phase: one active send, immediate `TRANSPORT_BUSY` from `try_lock` with no I/O, idempotent `close()` bounded by the close budget with `TIMEOUT_CLOSE`, `TRANSPORT_CLOSED` after close; no runtime is created and no finalizer closes anything |
| P05-004 | COMPLETED | `to_napi`/`to_result` map every core code and outcome into a stable `SENDUST:{json}` payload consumed by `ts/src/errors.ts`; credentials and bodies never enter a payload. A live relay reply mapped unchanged to `{ status, recipients, response }` |
| P05-005 | COMPLETED | This record. TypeScript facade, `ts/test/transport.test.ts` (4 tests), `tests/runtime/smoke.mjs` (5 checks each on Node.js 22.23.4, Bun 1.3.14, Deno 2.9.8), and a live Gmail send through the binding |

## 2. Acceptance checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Node-API 8 adapter compiles and calls only core APIs | PASS | `cargo clippy --workspace --all-targets --locked -D warnings`; the binding imports `sendust_core` only |
| Core remains Node-independent | PASS | `scripts/check-architecture.mjs` and the core-only test job; no `napi` in `sendust-core` |
| Buffers and strings are copied before async work | PASS | All inputs are owned before the first `await`; no borrowed reference crosses a suspension point |
| Lifecycle and error/result mappings are stable and tested | PASS | Vitest binding tests plus the runtime smoke on three runtimes |
| One active send, immediate BUSY, bounded idempotent close | PASS | Slot `try_lock` plus atomic phase; `close()` waits under the budget and reports `TIMEOUT_CLOSE` |
| No queue, pooling, retry, or GC-dependent close | PASS | No such path exists; close is explicit only |
| Redaction | PASS | Error payload carries code, category, message, and whitelisted details only; the auth-failure test asserts the password never appears |
| Runtime matrix | PASS | Node.js 22.23.4, Bun 1.3.14, Deno 2.9.8 all load the same addon and pass 5 checks; Deno needs `--allow-ffi --allow-read --allow-net` |
| Live relay proof | PASS | `node examples/send_email.mjs` against `smtp.gmail.com:465` returned `status: accepted` and `250 2.0.0 OK ... - gsmtp` |

## 3. Executed evidence

Run on macOS, `rustc 1.100.0`, Node 22.23.4, Bun 1.3.14, Deno 2.9.8:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
npm ci && npm run build:native
npm --workspace ./ts run typecheck && npm --workspace ./ts run lint && npm --workspace ./ts run test
node tests/runtime/smoke.mjs && bun tests/runtime/smoke.mjs && deno run --allow-ffi --allow-read --allow-net tests/runtime/smoke.mjs
node scripts/check-architecture.mjs && node scripts/check-docs.mjs && node scripts/check-secrets.mjs
```

Result: 94 core unit tests, 9 harness self-tests, 10 SMTP fault tests, 4 binding tests, and 5 runtime checks per runtime all pass; the live relay send was accepted.

## 4. Deferred (recorded, not claimed)

- Attachment payloads through the binding are P08 scope.
- The finalised public TypeScript declaration surface is P09 scope.
- Runtime × OS × CPU × ABI distribution and clean-install tests are P14 scope; this record claims only the three runtimes on the development host and the CI runner.
- Forced-GC and process-exit races are P10-001 scope.

## 5. Decision

**APPROVED.** The project owner reviewed this record and its evidence on
2026-11-14 and approved the PHASE-05 gate. The implementation was written by an
agent, so this approval is the required human review step, not a self-approval:
the reviewer is the project owner and the evidence is the CI run on the merged
commit plus the local commands above.

Approval covers P05 only. It does not approve P06, runtime/platform support, or
any later gate; a failed compile, binding, runtime, or checker result on `main`
reopens this gate.
