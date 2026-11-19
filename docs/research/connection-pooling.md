# Connection pooling research (P18-003)

**Status:** complete — research only. No code, test, package, or published
artifact changed.
**Date:** 2026-11-18.
**Baseline:** `@sendust/ts@0.1.0-rc.1` and ADR-0006 (single active send).

The MVP opens one connection per `send()` and allows one active send per
transport. This record asks whether connection reuse could ever be offered
without breaking that contract. Queueing, workers, persistence, scheduling, and
queue adapters are outside the product and are not considered here.

## 1. The contract any pooling proposal must preserve

| Rule | Source | Why it is non-negotiable |
| --- | --- | --- |
| One active send per transport, `BUSY` with no dial | ADR-0006, MVP scope | A second concurrent send must fail fast, not wait in an invisible queue |
| One connection per send today | README boundaries | The delivered behaviour and its tests |
| Explicit close, bounded by 125 s, idempotent | ADR-0006, lifecycle guide | Shutdown cannot depend on GC or on an idle timer |
| Timeouts: connect 10 s, operation 30 s, total 120 s | ADR-0007 | A reused connection must not extend a send's budget |
| `unknown` after `DATA` is never retried | Result contract | Retry after `unknown` can duplicate mail |
| No internal queue, worker, or scheduler | README boundaries | The consuming application owns orchestration |
| TLS verification always on | ADR-0003 | Reuse must not weaken session establishment |

## 2. Options evaluated

| Option | Shape | Assessment |
| --- | --- | --- |
| A — no pooling (current) | New TCP/TLS connection per send, closed after the final reply | Simplest, already shipped and tested. Cost is one handshake per message |
| B — reuse the connection within one transport between sends | After an accepted or rejected send, keep the session open and use `NOOP`/`RSET` before the next send; close on error or on `close()` | The only candidate that keeps `BUSY` and the absence of a queue. Still changes lifecycle, error, and close semantics; needs relay-lifetime evidence |
| C — pool of N connections with an internal queue | Acquire a connection from a pool; wait when all are busy | Rejected: it introduces an internal queue and a second active send, which the product forbids |
| D — caller-managed pool of transports | The application creates several `Transport` instances and routes work itself | Already possible today with no library change; the recommended pattern for concurrency |

## 3. What option B would need before an ADR

- **Session lifetime evidence.** How long popular relays keep an idle authenticated
  session open, and what they return for a `NOOP` on an expired one.
- **Failure mapping.** A reused connection can fail before `MAIL FROM`;
  that must surface as a connection error, never as a send outcome.
- **Budget semantics.** The total 120 s budget belongs to one send; a reused
  connection must not inherit or extend it.
- **`close()` semantics.** Close must drain an in-flight send exactly as now and
  must not wait for an idle session timeout.
- **Descriptor and memory bounds.** A held connection is a held descriptor for the
  transport's lifetime, which the leak probes would have to cover.
- **No queue by construction.** `BUSY` must still fire on a second concurrent
  `send()` even with a warm connection.
- **Support matrix.** Every claimed target and runtime must pass the reuse tests.

## 4. Decision

**Option B was adopted** by [ADR-0022](../architecture/decisions/0022-session-reuse.md)
on 2026-11-19, after ADR-0021 added the queue and pool that option B needs above
the core. A session is now kept after a definitive outcome, reset with `RSET`,
capped by message count and idle time, and dropped on any error or `unknown`
disposition. Option C remains rejected: the queue lives in the facade, and the
core never multiplexes one session.

The evidence section 3 asked for is recorded in ADR-0022: session lifetime is
bounded by `ReusePolicy`, failure mapping keeps a pre-`MAIL` failure a
connection error, and the send budget restarts for every message.

## 5. Evidence for the phase gate

- The record states that MVP remains one connection per send with no pooling and
  no automatic retry (sections 1 and 4).
- Any future pooling proposal has the lifecycle, concurrency, resource, and
  compatibility analysis it must carry (section 3).
- No queue architecture, adapter contract, or code change exists (sections 2
  and 4).
