# Benchmark: Sendust vs Nodemailer

**Date:** 2026-11-19.
**Command:** `npm run bench` (see `bench/run.mjs`, `bench/worker.mjs`, `bench/relay.mjs`).
**Baseline:** `@sendust/ts` at `main` after ADR-0021 and ADR-0022; Nodemailer `10.6.0`.

This is a controlled comparison on one machine, not a delivery claim and not a
provider benchmark. Both clients reported relay acceptance for every message;
no message failed.

## 1. Method

- **Relay.** A loopback TLS SMTP relay (`bench/relay.mjs`) using the committed
  test fixture certificate. It answers EHLO, MAIL, RCPT, DATA, RSET, NOOP, and
  QUIT, counts accepted connections and messages, and never throttles.
- **Payload.** Identical for both clients: `from`, one `to`, a subject, and a
  short `text` body.
- **Sendust.** One transport with `delivery.maxConcurrent` and
  `delivery.maxConnections` set to the workload concurrency, and
  `maxMessagesPerSession: 10_000` so the comparison measures connection reuse
  rather than the product's default 100-message session cap.
- **Nodemailer.** `pool: true`, `maxConnections` set to the same concurrency,
  `maxMessages: Infinity`.
- **Isolation.** Every (client, workload) pair runs in its own Node.js process,
  so RSS and CPU belong to one client. The harness alternates which client runs
  first.
- **Trials.** One warm-up batch, then three measured batches per pair; the
  throughput shown is accepted messages per second over the summed measured
  time, and latency percentiles pool every measured message.
- **Workloads.** 100 messages at concurrency 1, 5, 10, 25, 32, plus 1000 messages
  at concurrency 32. Sendust validates concurrency to 1–32, so 32 is its ceiling;
  both clients run at the same effective concurrency.

## 2. Environment

| Item | Value |
| --- | --- |
| Machine | MacBook Pro (Apple silicon), macOS |
| Runtime | Node.js v22.23.5 |
| Nodemailer | 10.6.0 |
| Network | loopback only (127.0.0.1), TLS with the test fixture |
| Relay location | in the same process as the client under test |

## 3. Results

| Workload | Concurrency | Messages | Sendust msg/s | Nodemailer msg/s | Sendust p95 | Nodemailer p95 | Sendust connections | Nodemailer connections |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Sequential | 1 | 100 | 5450.3 | 2397.8 | 17.4 ms | 44.7 ms | 4 | 1 |
| Low | 5 | 100 | 9946.1 | 2956.2 | 9.9 ms | 33.4 ms | 5 | 5 |
| Medium | 10 | 100 | 8243.7 | 2461.3 | 15.0 ms | 40.2 ms | 10 | 10 |
| High | 25 | 100 | 8206.9 | 2484.1 | 14.3 ms | 46.5 ms | 25 | 25 |
| Ceiling | 32 | 100 | 7937.9 | 3071.3 | 15.1 ms | 42.5 ms | 32 | 32 |
| Extended | 32 | 1000 | 13951.2 | 4924.9 | 80.4 ms | 207.9 ms | 39 | 32 |

Every run reported `ok` for all messages and zero failures for both clients:
300 accepted per 100-message workload and 3000 per extended workload.

**What the numbers show.** On this relay Sendust is roughly 2.2× (sequential) to
3.4× (concurrency 5) the throughput of Nodemailer, with a lower p95 in every
workload. On the extended run it is about 2.8×. Sendust's connection count is
close to the concurrency in each workload; the sequential run used 4 connections
for 320 messages and the extended run 39 for 3020, so a few sessions were
re-established. That is visible in the data rather than explained away.

## 4. What this does not show

- **Not a provider measurement.** The relay never throttles, never rate-limits,
  and never fails. Real providers, real quotas, and real latency are untested.
- **Not a delivery claim.** Every result is relay acceptance; nothing here says a
  mailbox received the message.
- **Not absolute throughput.** The relay shares a process and a CPU with the
  client, so both clients are measured under the same constraint but neither at
  its ceiling. The comparison is relative.
- **Not a full matrix.** One machine, one Node.js version, one relay type, short
  plain-text messages. TLS cost is RSA with the fixture certificate.
- **The session count needs an explanation.** Sendust dialed more than once per
  transport in two workloads. The cause is not yet established; it is recorded
  here rather than hidden.

## 5. Reproducing

```text
npm ci
npm run build:native
npm --workspace ./ts run build
npm run bench
```

The harness exits non-zero if any run reports a failure, and prints the table in
section 3.
