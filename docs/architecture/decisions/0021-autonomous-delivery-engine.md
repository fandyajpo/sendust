# ADR-0021 — Autonomous delivery engine: queue, pool, and adaptive concurrency

**Status:** ACCEPTED (project owner decision, 2026-11-18; supersedes ADR-0006's
"no pool, no queue" clause and the product boundary in the README).
**Related:** [audit](../../audits/2026-11-18-autonomous-delivery-audit.md),
ADR-0005 (recipient partial failure), ADR-0006 (superseded clause),
ADR-0007 (timeout budgets), ADR-0013 (error taxonomy), ADR-0017 (native engine).

## 1. Context

The published MVP runs one send at a time per transport and rejects a concurrent
`send()` with `TRANSPORT_BUSY`. That was a deliberate scope decision, and the
[audit](../../audits/2026-11-18-autonomous-delivery-audit.md) shows what it
costs: no queue, no pool, no connection reuse, no retry, no rate limiting, and a
fresh TCP + TLS + AUTH round trip for every message.

A caller who submits a thousand messages therefore has to build the concurrency
layer itself, which is the complexity the product exists to remove.

## 2. Decision

Keep the native core exactly as it is — **one native transport owns one SMTP
session, one send at a time** — and add the autonomous layer **above** it, in the
runtime-neutral TypeScript facade.

That split preserves the invariant that makes the SMTP code safe (no
multiplexing on one connection, ever) while letting the facade own everything
that is policy rather than protocol:

| Component | Responsibility | Location |
| --- | --- | --- |
| Message queue | Accept every `send()`, bound it by count and bytes, keep one promise per message | `ts/src/engine.ts` |
| Connection pool | Acquire/release native transports, cap their number, create lazily, release on every path | `ts/src/engine.ts` |
| Adaptive concurrency | Start at one, grow on sustained success, halve on throttling, with a cooldown | `ts/src/engine.ts` |
| Rate limiter | Token bucket over messages per second, off by default | `ts/src/engine.ts` |
| Response classification | Success / transient / permanent / unknown, and what may be retried | `ts/src/policy.ts` |
| Retry and recovery | Exponential backoff with jitter, attempt budget, never after `unknown` | `ts/src/policy.ts`, `ts/src/engine.ts` |
| Shutdown | Stop accepting, reject queued work with `TRANSPORT_CLOSED`, abort active work, close every native transport | `ts/src/engine.ts` |

## 3. Non-negotiable rules

1. `accepted` still requires a positive final `DATA` reply. The engine never
   rewrites an outcome.
2. `unknown` is **never** retried. A lost final reply is an uncertain delivery,
   not a failure to retry.
3. A `rejected` result is retried only when its final reply is `4xx`. RFC 5321
   makes a `4xx` at the end of `DATA` a non-acceptance, so the retry cannot
   duplicate a message.
4. `535`/`AUTH_*` stops the transport: the credentials are wrong, and retrying
   hammers the relay. Queued work fails fast with the same error.
5. No retry bypasses throttling: a `421` halves concurrency and starts a
   cooldown; nothing rotates connections to evade a limit.
6. Every promise settles. There is no path where a queued message is dropped
   without rejection.

## 4. Configuration

Automatic management is the default; the new options are overrides, not
requirements.

```ts
createTransport({
  host, port, secure,
  delivery: {
    maxConcurrent: 8,        // 1..32
    maxConnections: 8,       // 1..32, default = maxConcurrent
    maxQueue: 10_000,        // 1..1_000_000
    maxQueueBytes: 64 << 20, // 1..1 GiB
    messagesPerSecond: 0,    // 0 = unlimited
    maxAttempts: 3,          // 1..10
    retryBaseMs: 250,        // 0..60_000
    retryMaxMs: 30_000,      // 0..600_000, ≥ retryBaseMs
  },
});
```

A value outside its range is rejected with `VALIDATION_INVALID_OPTION` before any
message is accepted, in the same spirit as the existing option checks.

## 5. Consequences

**Positive**

- `Promise.all(1000 × send())` works. The queue absorbs the burst and the pool
  runs up to `maxConcurrent` native transports.
- Transient failures recover without the caller writing retry logic, and
  permanent failures still surface immediately.
- Throughput and queue depth become observable (`transport.stats`).

**Negative / accepted**

- The default behavior changes: a second concurrent `send()` is queued instead
  of throwing `TRANSPORT_BUSY`. That is a **major version change** under the
  versioning policy, and `TRANSPORT_BUSY` becomes reachable only from the native
  layer.
- `close()` now rejects queued messages with `TRANSPORT_CLOSED`. The policy is
  abort-on-close, matching the existing bounded-close contract; a `drain` policy
  is deferred.
- One connection per *concurrent send* is still created: the pool reuses native
  transports, not sockets. **Persistent connection reuse is explicitly out of
  scope for this ADR** and needs a core change (a session that survives a
  transaction with `RSET`, idle timeout, and post-error recovery).

## 6. Evidence this ADR requires

- The full suite still passes, including every existing `transport.test.ts` case
  (updated where the contract changed, never deleted to hide a regression).
- New tests cover: 100 concurrent sends, queue overflow, transient retry then
  success, permanent non-retry, `unknown` non-retry, rate limiting, isolated
  failures, and close with queued work.
- Typecheck, lint, and build pass on the pinned toolchain.
- No benchmark claim is made until a benchmark exists; the harness and workloads
  are specified before any number is published.

## 7. Deferred

| Item | Reason |
| --- | --- |
| Persistent connection reuse | Needs a core session-lifetime change; a facade-only claim would be false |
| Provider quota tables | SMTP does not advertise daily/hourly quotas; guessing violates the "no fabricated limits" rule |
| Persistent queue adapters | Needs a durability contract; the bounded in-memory queue is enough for a library |
| `drain` shutdown policy | Needs a documented interaction with in-flight cancellation; abort-on-close is already safe |
| Nodemailer benchmark | Blocked on the engine existing; the workloads are fixed in advance |
| SMTPUTF8, `AbortSignal`, extra platforms | Own ADRs, unchanged by this decision |
