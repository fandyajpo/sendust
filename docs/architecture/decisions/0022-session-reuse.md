# ADR-0022 — Reuse one authenticated SMTP session across messages

**Status:** ACCEPTED (project owner decision, 2026-11-19; supersedes ADR-0006's
"no reuse" clause and adopts option B of the
[connection pooling research](../research/connection-pooling.md)).
**Related:** ADR-0006 (superseded clause), ADR-0007 (timeout budgets),
ADR-0017 (native engine), ADR-0021 (autonomous delivery engine).

## 1. Context

The audit ([2026-11-18](../audits/2026-11-18-autonomous-delivery-audit.md))
found that every `send()` opened a fresh TCP connection, TLS handshake, and AUTH
exchange, because `SmtpTransport::run` called `Session::open` unconditionally and
`QUIT` at the end. On a real relay the handshake dominates the cost of a small
message, so throughput was bounded by round trips rather than by us.

ADR-0021 added a queue and a pool above the core, but a pool of transports still
paid one handshake per message: it reused objects, not sessions.

## 2. Decision

`SmtpTransport` keeps its session between sends.

- After a **definitive** outcome — `accepted` or `rejected` — the adapter closes
  the transaction with `RSET` and keeps the authenticated session for the next
  message.
- After **any error**, or an **`unknown`** disposition, the session is dropped.
  Its protocol state cannot be trusted, and reuse must never turn an uncertain
  delivery into a duplicated one.
- A session serves at most `ReusePolicy::max_messages` messages (default 100)
  and is not reused after `ReusePolicy::idle` of inactivity (default 30 s). Both
  are configurable through `delivery.maxMessagesPerSession` and
  `delivery.sessionIdleMs`.
- A reused session gets a **fresh send budget**: `Connection::begin_transaction`
  restarts `SendDeadlines`, so a later message never inherits the total deadline
  of the send that opened the session (ADR-0007).
- `close()` sends `QUIT` on a retained session, bounded by the close budget.
  An in-flight send that close cancels still drops its session abruptly; that is
  the existing cancellation path, unchanged.
- One transaction runs at a time per session. Concurrency remains the pool's
  job (ADR-0021): the pool creates more sessions, it never multiplexes one.

## 3. Why not the alternatives

| Option | Why it was not chosen |
| --- | --- |
| A — connection per send | The shipped behavior; it costs a handshake per message and was the audit's CRITICAL finding |
| C — pool with an internal queue in the core | Duplicates ADR-0021's queue inside the native layer and makes the lifecycle contract ambiguous |
| Reuse after an error or `unknown` | A half-completed transaction can desynchronize the stream; the risk of a duplicated message outweighs the saved handshake |

## 4. Consequences

**Positive**

- A second message on the same transport costs `MAIL`, `RCPT`, `DATA`, `RSET`
  instead of connect + TLS + AUTH. Throughput on small messages is no longer
  handshake-bound.
- Fewer TLS handshakes also mean fewer CPU cycles and fewer relay connections,
  which matters for providers that meter concurrent connections.

**Negative / accepted**

- A held session is a held socket. The descriptor-leak probes now cover a
  transport that keeps a session open, and `close()` is the only thing that
  releases it; a caller that never closes keeps one connection per transport.
- A relay that silently expires idle sessions makes the first reused send fail.
  That failure surfaces as a normal connection error, is retried by the delivery
  engine (ADR-0021) only when it is pre-DATA and transient, and drops the
  session. `sessionIdleMs` keeps the window conservative.
- `SmtpTransport` is no longer `Clone`, because it owns a live session.

## 5. Evidence

- `crates/sendust-core/tests/smtp_reuse.rs`: two sends produce one `EHLO` and
  two `MAIL`; a capped session re-dials and quits; an `unknown` disposition is
  never reused; `shutdown()` sends `QUIT`.
- The full Rust suite, clippy, and the five-target CI artifacts must pass.
- The facade's 100-message burst test asserts the relay accepts at most one
  connection per pooled transport, not one per message.

## 6. Deferred

| Item | Reason |
| --- | --- |
| `NOOP` probing before reuse | `RSET` already proves the session alive; a probe would add a round trip |
| Reuse across transports | A pool already covers it; sharing one session between transports would reintroduce BUSY |
| Server-initiated expiry handling | Needs per-provider idle evidence; `sessionIdleMs` is the conservative answer for now |
