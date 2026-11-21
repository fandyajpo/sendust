# Real-relay latency: analysis and the two fixes

**Date:** 2026-11-21.
**Baseline measured on:** `smtp.gmail.com:465`, 100 messages, concurrency 5, by the
project owner's own harness (`~/Documents/Personal/sendust-test`).
**Compared with:** Nodemailer 10.6.0, same relay, same payload, same concurrency.

## 1. The measurement

| Metric | Sendust (before) | Nodemailer |
| --- | ---: | ---: |
| Duration | 80 843 ms | 75 007 ms |
| Throughput | 1.24/s | 1.33/s |
| Average latency | 3 991 ms | 3 641 ms |
| p95 | **11 869 ms** | 5 286 ms |
| p99 | **13 414 ms** | 5 665 ms |
| CPU user | 93 ms | 1 542 ms |
| Success | 100/100 | 100/100 |

Throughput was within 7%, but the **tail** was more than twice as long. Both
clients report relay acceptance only; this is not a delivery measurement.

## 2. What the shape says

At concurrency 5, 100 messages and roughly 4 s per message, the run should take
about 80 s — which both took. So the ceiling is the relay, not the client. The
difference is **when** each client reaches that ceiling:

- Nodemailer opens up to `maxConnections` immediately, so all five lanes are busy
  from the first second and no message waits behind another for long.
- Sendust's engine started at **one** send and grew **one slot per eight
  consecutive successes** (ADR-0021). Reaching five slots takes roughly 32
  completed messages, and reaching eight takes 56. For most of a 100-message
  burst, most of the pool sat idle, so later messages waited in the queue — and
  queue wait is exactly what lands in the p95 and p99.

That design was chosen to be gentle with an unknown relay. It is the wrong
default for a library that a caller hands a burst to, because the backoff
already exists: any transient failure halves the level and starts a cooldown.

A second, smaller cost: after every completed transaction the core sent `RSET`.
After a successful `DATA` the session is already idle, so that was a full round
trip per message spent on nothing — a few percent of a 4 s message, more on a
fast relay.

## 3. The two changes

1. **Demand sets the level** (`ts/src/engine.ts`, `#meetDemand`). Before each
   scheduling pass, concurrency rises to `min(maxConcurrent, active + queued)`.
   A burst therefore uses the whole configured pool at once. `#throttle` still
   halves it and holds a 1 s cooldown after a temporary failure or a `4xx`, so a
   relay that pushes back still sees Sendust back off.
2. **No `RSET` after a completed transaction** (`crates/sendust-core`, `retain`).
   `RSET` is now sent only when an envelope was left open — every recipient
   refused, or `DATA` refused — and a session that received `421` is never reused
   at all, because the relay is closing the channel.

Both are behaviour changes a caller cannot observe except in timing and in the
wire trace: no public API, result, or error code changed.

## 4. What is verified

- `crates/sendust-core/tests/smtp_reuse.rs`: two accepted sends produce **one**
  `EHLO`, two `MAIL` and **zero** `RSET`; a refused envelope produces one `RSET`
  before reuse; `unknown` is never reused; `shutdown` quits.
- `ts/test/engine.test.ts`: eight queued sends against a stalled relay open
  eight sessions and report concurrency 8.
- Full suite: Rust workspace, clippy `-D warnings`, `cargo fmt --check`;
  TypeScript typecheck, 71 tests, format, and all four `npm run check` gates.

## 5. What is not yet measured

**The real-relay comparison has not been re-run.** These numbers came from the
owner's Gmail account, and repeating them means sending another 100+ messages.
The expected effect is that Sendust's p95 and p99 fall toward Nodemailer's while
CPU stays where it is, but that is a prediction, not a result, and it is not
recorded here as one.

To measure it, from `~/Documents/Personal/sendust-test`:

```text
git -C ~/Documents/Personal/sendust pull --ff-only      # 62d6f36 or later
cd ~/Documents/Personal/sendust-test
npm i @sendust/ts@github:fandyajpo/sendust                 # or the next published version
npm run build && npm start
# then POST the same benchmark request as before, concurrency 5, count 100
```

`1.0.1` carries both fixes and is the published `latest`; `1.0.0` does not have
them.

## 6. Recommended production configuration

The defaults now behave well for bursts. Two things are still worth setting
deliberately:

- `delivery.maxConcurrent` should match what the provider tolerates, not the
  machine: Gmail accepts a handful of parallel sessions per account, so 4–8 is
  reasonable; higher mostly buys `421`s.
- `delivery.messagesPerSecond` is the honest way to respect a known quota; the
  adaptive level reacts to pushback but cannot know a daily cap.

## 7. Remaining limitations

- Per-message cost is dominated by the relay's `DATA` handling (~4 s here), so
  client-side wins are bounded by concurrency, not by micro-optimisation.
- Provider quotas are still not modelled; only observed pushback is.
- `p99` will always include queue wait whenever the burst exceeds the pool; that
  is a property of a bounded pool, not a defect.
