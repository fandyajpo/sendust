# Lifecycle, timeouts, concurrency, and limits

**Status:** user guide for the MVP. Numbers are copied from
`crates/sendust-core/src/config/mod.rs` and mirrored in `ts/src/limits.ts`; the
facade's pre-flight check and the architecture checker keep the two in step.

## 1. Lifecycle

```text
        close()                  drain complete
  open ─────────► closing ──────────────────────► closed
    │                 │
    │ send()          │ send() ─► TRANSPORT_CLOSED
    ▼                 ▼
  one send                       no new work

  open ── AUTH_FAILED / AUTH_UNSUPPORTED ──► failed ── close() ──► closed
```

`transport.state` is a synchronous, read-only snapshot:
`"open"`, `"closing"`, `"closed"`, or `"failed"`.

- `open` — accepts exactly one send at a time.
- `closing` — `close()` has been called. New sends are refused with
  `TRANSPORT_CLOSED`; an in-flight send is being cancelled or drained.
- `closed` — no further work; `send()` rejects with `TRANSPORT_CLOSED`.
- `failed` — the engine stopped on a condition that does not heal: rejected or
  unsupported credentials (`AUTH_FAILED`, `AUTH_UNSUPPORTED`). Every `send()`
  rejects with that same error and dials nothing, and `stats.state` reports
  `failed` too, so a health check keyed on it does not read `open` while every
  send fails. There is no reset in process; rotate the credential and build a new
  transport. `close()` still works, and settles `closed`.

A send that was cancelled **before** the message body was transmitted rejects
with `TRANSPORT_CLOSED`. If the body may already have reached the relay, the
outcome is `unknown` instead — close never rewrites that uncertainty into a
definite success or failure.

## 2. Close

```js
try {
  // ... sends ...
} finally {
  await transport.close();
}
```

- **Required.** A transport holds a connection; closing it is explicit.
- **Idempotent.** Repeated calls resolve once the transport is closed, and
  concurrent calls share the same drain.
- **Bounded.** The drain is capped at `closeMs` (125 s by default). If the active
  send does not finish in time, `close()` rejects with `TIMEOUT_CLOSE` and the
  transport is `closed` anyway.
- **Never GC-driven.** No finalizer, no process-exit hook. Do not rely on the
  garbage collector to close connections.

## 3. Concurrency and the queue

`send()` is **queued**. A transport runs up to `delivery.maxConcurrent` sends at
once (8 by default) over at most `delivery.maxConnections` native sessions, and
each session runs one SMTP transaction at a time.

| Situation | Behaviour |
| --- | --- |
| Many concurrent `send()` calls | Queued and scheduled; results settle per message |
| Queue at `delivery.maxQueue` or `maxQueueBytes` | Rejects with `LIMIT_QUEUE_FULL`; nothing is silently dropped |
| `send()` while `closing` or `closed` | Rejects with `TRANSPORT_CLOSED` |
| `send()` while `failed` | Rejects with the error that stopped it — `AUTH_FAILED` or `AUTH_UNSUPPORTED` — and dials nothing |
| Two `close()` calls | Both resolve; one shutdown |
| Two transports | Independent queues, pools, and limits |

Concurrency rises to the work already waiting, up to `delivery.maxConcurrent`. A
`4xx` final reply or a transient start failure halves it and starts a cooldown.
Only transient failures are retried; `unknown` never is.

A session that sits idle for `delivery.sessionIdleMs` (30 s by default) is closed
politely and dropped from the pool, so a transport that stops sending does not
hold a relay connection open until shutdown.

## 4. Timeouts

All budgets are finite, may only be **lowered**, and default to their maximum.

| Option | Default | Allowed range | Covers |
| --- | --- | --- | --- |
| `timeouts.connectMs` | 10 000 ms | 1000–10000 | DNS, TCP connect, greeting, `EHLO`, TLS handshake (or STARTTLS upgrade) |
| `timeouts.operationMs` | 30 000 ms | 1000–30000 | A single awaited SMTP reply, including AUTH and each `MAIL`/`RCPT`/`DATA` reply |
| `timeouts.totalMs` | 120 000 ms | 1000–120000 | The whole send, from reservation to final reply or outcome |
| `timeouts.closeMs` | 125000 ms | `totalMs`–125000 | The close drain |

Ordering rules enforced at `createTransport`:

- `connectMs` and `operationMs` must not exceed `totalMs`.
- `closeMs` must be at least `totalMs`.
- Any value outside its range, or a value that breaks the ordering, fails with
  `VALIDATION_INVALID_TIMEOUT`.

Lowering is the only adjustment. A partial object is merged with the defaults,
so `timeouts: { totalMs: 5000, connectMs: 1000, operationMs: 1000, closeMs: 5000 }`
is valid; raising any value above its default is rejected.

## 5. Hard caps

| Limit | Default = maximum | What it bounds |
| --- | --- | --- |
| `limits.maxRecipients` | 100 | `to` + `cc` + `bcc` combined |
| `limits.maxAttachmentBytes` | 10 MiB (10 485 760) | One attachment, raw bytes |
| `limits.maxTotalAttachmentBytes` | 20 MiB (20 971 520) | All attachments of one message, raw bytes |
| `limits.maxWireBytes` | 30 MiB (31 457 280) | The serialized message, exactly as written to the socket |

- The caps are **inclusive**: exactly 10 MiB is accepted, 10 MiB + 1 byte is not.
- Buffer attachments are checked **before** any byte is copied; an oversize
  buffer fails with `LIMIT_ATTACHMENT_BYTES` / `LIMIT_ATTACHMENT_TOTAL` without
  duplicating the payload.
- Path attachments are sized while reading: the per-file cap triggers
  `LIMIT_ATTACHMENT_BYTES`, the aggregate `LIMIT_ATTACHMENT_TOTAL`.
- The wire cap is checked on the exact serialized length, so an accepted message
  is guaranteed to fit the byte count that was measured.

Lowering a cap is allowed; raising it is not.

## 6. Operational guidance

- **Do not retry an `unknown` outcome.** Sendust retries only a `4xx` final reply
  or a connect or operation timeout before `DATA`, inside that message's own send
  budget. An `unknown` disposition is never retried; decide about it at the
  caller level, with idempotency in mind
  (see the [quick start](quick-start.md#unknown-is-not-a-retry-signal)).
- **Close during shutdown.** Close every transport in the shutdown path, and
  await it. The 125 s close budget is above the 120 s send budget so a maximal
  send can still drain.
- **One transport per service, not per request.** The engine already queues and
  pools; a transport per request multiplies connections and defeats the pool.
- **A failed transport is not a closed one.** After `AUTH_FAILED` or
  `AUTH_UNSUPPORTED`, `state` and `stats.state` report `failed`, and every send
  rejects with that code. Rotate the credential and build a new transport; there
  is no in-process reset, and retrying on the same transport only repeats the
  error.
- **Lower timeouts for interactive paths.** A relay that stalls will otherwise
  hold a request for the full 120 s total budget.
- **Watch the caps, not the provider.** Rejecting a 12 MiB attachment locally is
  cheaper and more predictable than discovering the relay's own limit.

## 7. Related reading

- [Public API contract](../architecture/public-api.md) — state, close, concurrency.
- [Quick start and semantic contract](quick-start.md) — result statuses.
- [Error model](../architecture/error-model.md) — every timeout and limit code.
