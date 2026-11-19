# Autonomous delivery audit — concurrency, pooling, and provider limits

**Date:** 2026-11-18.
**Baseline:** `@sendust/ts@0.1.0-rc.1` at `0d9b20`-era `main` (see the commit this
document lands on).
**Scope:** the whole delivery path — `createTransport()` → `send()` → native
session → SMTP transaction → close — plus the tests, package surface, and build
configuration that surround it.
**Decision record:** [ADR-0021](../architecture/decisions/0021-autonomous-delivery-engine.md).

This audit answers one question: what stands between the published MVP and a
delivery engine that accepts large concurrent workloads without the caller
managing connections, concurrency, or retries? It classifies every finding it
found, cites the file and line that proves it, and separates what was measured
from what was reasoned.

Nothing here is a defect against the published artifact: the MVP behaves exactly
as its approved contract says. The findings are against the contract itself.

## 1. Root cause of `transport is busy`

The string is produced in exactly one place.

- `crates/sendust-napi/src/lib.rs:262-268` — `Transport::send` takes
  `active: tokio::sync::Mutex<Active>`; if the slot is not `Active::Idle` it
  returns `ErrorCode::TransportBusy`.
- `crates/sendust-napi/src/lib.rs:270-287` — the same code is also returned when
  `slot.try_lock()` fails or the transport has already been moved out, which can
  only happen while a send is in flight.
- `crates/sendust-core/src/errors/` maps `TransportBusy` to the `busy` category;
  `ts/src/errors.ts:77` turns it into `SendustBusyError`, and
  `ts/src/results.ts` never sees it because it is an error, not an outcome.

The facade adds no check of its own: `ts/src/transport.ts:258-271` calls the
native `send()` directly. So the rejection is the native single-flight guard, and
it is *by design*: [ADR-0006](../architecture/decisions/0006-lifecycle-single-send.md)
freezes "at most one active `send`" and "a concurrent `send` fails immediately …
never queued or delayed".

**Consequence:** a caller who does `Promise.all(1000 × send())` gets one accepted
send and 999 `TRANSPORT_BUSY` rejections. The documented workaround
([public-api.md §8](../architecture/public-api.md)) is "create multiple
transports", which puts pool management back on the caller — the opposite of the
product's own goal.

## 2. Findings

Severity: **CRITICAL** (message loss, protocol corruption, deadlock), **HIGH**
(concurrency, reliability, or performance defect), **MEDIUM** (functional but
needs improvement), **GREEN** (correct and production-ready).

### 2.1 Concurrency

| # | Sev | Finding | Evidence |
| --- | --- | --- | --- |
| C-1 | HIGH | One active send per transport; concurrent sends are rejected, never queued. | `crates/sendust-napi/src/lib.rs:262-268`; `ts/src/transport.ts:204-207` |
| C-2 | HIGH | No internal queue exists at any layer. `send()` either starts I/O or throws. | `ts/src/transport.ts:258-271`; no queue module in `ts/src/` |
| C-3 | MEDIUM | The single-flight guard has three separate `TransportBusy` exits (active flag, `try_lock`, empty slot). They are consistent today but duplicate the same decision in three places. | `crates/sendust-napi/src/lib.rs:264-286` |
| C-4 | GREEN | The active flag is released on every path, including a cancelled task, and `idle.notify_waiters()` wakes `close()`. No permanently locked transport was found. | `crates/sendust-napi/src/lib.rs:331-336`, `:357-391` |
| C-5 | GREEN | A failed request cannot corrupt an unrelated one: the native transport holds configuration only, so there is no cross-request mutable state beyond the active flag. | `crates/sendust-napi/src/lib.rs:124-133` |
| C-6 | MEDIUM | `Active::Sending(None)` — the window between claiming the slot and storing the abort handle — is handled, but only by a second `notify_waiters()`. It is subtle and untested directly. | `crates/sendust-napi/src/lib.rs:318-325` |

### 2.2 Connections

| # | Sev | Finding | Evidence |
| --- | --- | --- | --- |
| N-1 | **CRITICAL for throughput** | Every `send()` opens a **new TCP connection, new TLS handshake, and a new AUTH exchange**. `SmtpTransport` stores configuration and progress only; `run()` calls `Session::open(config)` each time and `quiet_quit()` at the end. There is no connection reuse anywhere. | `crates/sendust-core/src/transports/smtp/mod.rs:207-210`, `:261-270`, `:485-487` |
| N-2 | MEDIUM | The native `SmtpTransport` instance is *reused* across sends (the slot hands it back), which suggests reuse but proves nothing about the socket: the socket is created and destroyed inside `run()`. | `crates/sendust-napi/src/lib.rs:279-286`, `:339-343` |
| N-3 | MEDIUM | `maxMessagesPerConnection`, connection expiry, and idle TTL do not exist, because nothing outlives one send. | grep for `keepalive`, `reuse`, `pool` in `crates/sendust-core/src` returns nothing outside comments |
| N-4 | GREEN | TLS verification is mandatory and cannot be disabled; an unadvertised STARTTLS fails rather than downgrading. | `crates/sendust-core/src/transports/smtp/mod.rs:272-278` |
| N-5 | GREEN | Socket and capability state live in `Connection`, one per session, with bounded read/write timeouts. | `crates/sendust-core/src/transports/smtp/client/connection.rs` |

### 2.3 SMTP outcome classification

| # | Sev | Finding | Evidence |
| --- | --- | --- | --- |
| R-1 | HIGH | Classification is binary: positive vs refused. A `421`, `450`, `451`, `452`, `535`, `550`, and `554` all become the same `rejected` outcome (or the same `unexpected_response` error) depending only on the stage. Nothing distinguishes "try again later" from "never". | `crates/sendust-core/src/transports/smtp/mod.rs:464-481` |
| R-2 | MEDIUM | A `421` at `RCPT` is special-cased to stop the envelope, but the recipients are reported `unknown` inside a `rejected` *outcome*, with no response attached, so a caller cannot tell why. | `crates/sendust-core/src/transports/smtp/mod.rs:324-329`, `:340-343` |
| R-3 | GREEN | Nothing is ever reported `accepted` without a positive final `DATA` reply, and a lost final reply is `unknown`. | `crates/sendust-core/src/transports/smtp/mod.rs:366-402` |
| R-4 | GREEN | `unknown` is never retried by any code path. | `crates/sendust-core/src/transports/smtp/mod.rs:393-401` |

### 2.4 Retry, backoff, and rate limiting

| # | Sev | Finding | Evidence |
| --- | --- | --- | --- |
| T-1 | HIGH | There is no retry, no backoff, no jitter, and no retry budget anywhere in the tree. Every transient failure reaches the caller as an outcome or an error. | grep for `backoff`, `jitter`, `retry` in `crates/` and `ts/src` finds only doc text |
| T-2 | HIGH | There is no rate limiter and no provider policy. Two sends in a row are emitted as fast as the relay answers. | no limiter module exists |
| T-3 | MEDIUM | Timeouts are per send, not per window, so a caller cannot express "500 messages/minute". | `ts/src/types.ts:61-76` |

### 2.5 Resource management and memory

| # | Sev | Finding | Evidence |
| --- | --- | --- | --- |
| M-1 | GREEN | Message buffers are copied synchronously into owned memory before any await, so a caller may mutate its arrays immediately. | `ts/src/transport.ts:134-152`, `:162-163` |
| M-2 | GREEN | Attachment caps are enforced *before* copying, so an oversize payload never gets duplicated. | `ts/src/transport.ts:159-162`; `ts/src/limits.ts` |
| M-3 | MEDIUM | Each send allocates a fresh socket, TLS buffers, and a session; with no reuse there is no way to bound steady-state allocations under load. | consequence of N-1 |
| M-4 | GREEN | Descriptor leaks across refused connections are covered by a dedicated probe that fixed a real CI flake. | `crates/sendust-core/tests/leaks.rs` |

### 2.6 API, tests, and build

| # | Sev | Finding | Evidence |
| --- | --- | --- | --- |
| A-1 | GREEN | Unknown transport options are rejected, so a typo cannot be silently ignored. | `ts/src/transport.ts:50-57`, `:80-85` |
| A-2 | MEDIUM | The surface is frozen so tightly that `pool: 4` does not type-check; adding automatic management needs new optional configuration and a major version. | `ts/test/types.test.ts:91-111` |
| A-3 | GREEN | The fake relay and TLS fixtures exercise accepted, rejected, dropped-final-reply, TLS failures, BUSY, and close. | `ts/test/transport.test.ts:84-148` |
| A-4 | MEDIUM | No test runs more than two concurrent sends, so nothing measures the concurrency ceiling. | `ts/test/transport.test.ts:505-546` |
| A-5 | MEDIUM | No benchmark exists against Nodemailer, and no harness measures throughput, latency percentiles, or connection counts. | no `bench` or `benchmark` script in `package.json` or `ts/package.json` |
| A-6 | GREEN | The build, lint, typecheck, and test commands are pinned and run in CI on five targets. | `.github/workflows/ci.yml`; `docs/engineering/toolchain-baseline.md` |

## 3. Execution trace (as it is today)

```
createTransport(options)
  └─ loads the addon, constructs one native Transport (config only)   ts/src/transport.ts:225-238
send(message)
  ├─ facade copies attachments + builds the native message record      ts/src/transport.ts:258-266
  ├─ native: claim the active slot, else TRANSPORT_BUSY                napi/src/lib.rs:262-268
  ├─ native: take the SmtpTransport from the slot                      napi/src/lib.rs:270-287
  └─ Delivery::send
        └─ SmtpTransport::run
              ├─ Session::open  → TCP connect + TLS + EHLO            smtp/mod.rs:268-270
              ├─ (STARTTLS upgrade + re-EHLO)                          smtp/mod.rs:272-277
              ├─ AUTH                                                  smtp/mod.rs:279-289
              ├─ MAIL FROM                                             smtp/mod.rs:291-308
              ├─ RCPT TO × n                                           smtp/mod.rs:314-338
              ├─ DATA → body → final reply                             smtp/mod.rs:345-402
              └─ QUIT (best effort)                                    smtp/mod.rs:485-487
  └─ native: return the transport to the slot, release the active flag  napi/src/lib.rs:331-343
close()
  └─ native: swap to CLOSING, abort the active send, wait ≤ closeMs     napi/src/lib.rs:357-391
```

There is no queue, no pool, and no reuse in that trace. The "release" step hands
back a configuration object, not a connection.

## 4. What the audit says to build

1. **A queue above the native transport.** The native layer must keep its
   single-flight guarantee — it is what makes the protocol safe — and the facade
   must stop surfacing `TRANSPORT_BUSY` to callers. One queue, one promise per
   message, results isolated per message.
2. **A pool of native transports.** Each holds one connection at a time. The
   pool bounds how many exist, creates them lazily, and releases them on every
   path, including cancellation.
3. **Classification, then retry.** SMTP replies must be classified before
   anything is retried, `unknown` must never be retried, and a `4xx` after `DATA`
   is retryable only because RFC 5321 makes it a non-acceptance — never a
   duplicate.
4. **A limiter and an adaptive controller.** Conservative by default, growing on
   sustained success, halving on throttling, with hysteresis so it cannot
   oscillate.
5. **An honest close.** A documented policy for pending work, no unresolved
   promise, bounded by the existing close budget.

**Deferred, with reasons**

- **Persistent connection reuse** needs a core change: a session that survives a
  transaction (`RSET`, idle timeout, post-error recovery). Adding it to the
  facade alone would be a lie. Deferred to its own ADR and phase.
- **Provider-specific quota tables** are not added: SMTP does not advertise
  daily or hourly quotas, and guessing them would violate the product's own
  "never fabricate limits" rule.
- **Persistent queue adapters** are deferred: an in-memory bounded queue is
  enough for a library, and an adapter needs its own durability contract.
- **Nodemailer benchmarks** are deferred until the engine exists to benchmark;
  the harness is described in section 6 of ADR-0021 so the measurement is fixed
  before any claim is made.

## 5. Classification summary

| Severity | Count | Items |
| --- | --- | --- |
| CRITICAL | 1 | N-1 (a full handshake and AUTH per message) |
| HIGH | 6 | C-1, C-2, R-1, T-1, T-2 |
| MEDIUM | 11 | C-3, C-6, N-2, N-3, R-2, T-3, M-3, A-2, A-4, A-5 |
| GREEN | 12 | C-4, C-5, N-4, N-5, R-3, R-4, M-1, M-2, M-4, A-1, A-3, A-6 |

No CRITICAL correctness defect was found: nothing in the current code can lose a
message silently, report `accepted` without a positive `DATA` reply, or corrupt
an unrelated transaction.

## 6. Limits of this audit

- It is a static and behavioural review, not a load test. No throughput number is
  claimed here; section 7 of the request's benchmark plan is deferred until the
  engine is benchmarkable.
- Windows and musl targets were not re-tested for this audit; their status is the
  published PHASE-14 evidence.
- Real relays were not contacted. Every observation above is reproducible against
  the loopback fixtures in `tests/fixtures/tls`.
