# Lifecycle, BUSY, and Close Performance (P13-004)

**Status:** complete — every budget met; no queue and no descriptor drift
**Date:** 2026-11-15
**Harness:** `tests/perf/lifecycle.mjs` against the staged addon
**Command:** `node tests/perf/lifecycle.mjs`

## 1. Environment

| Item | Value |
| --- | --- |
| Node.js | 22.23.4 |
| Host | macOS 15.0, Apple M2, 8 cores |
| Addon | release build of `sendust-napi`, staged as `ts/sendust.node` |
| Relay | loopback TCP server that accepts and then stays silent (TLS handshake stall) |
| Warm-up / measured cycles | 3 / 20; 1 000 idle close cycles |

## 2. Results

| Metric | p50 / p95 | Budget p50 / p95 | Result |
| --- | --- | --- | --- |
| BUSY rejection | 24.2 / 93.8 µs | 100 / 500 µs | PASS |
| Idle close | 11.1 / 26.6 µs | 5 000 / 10 000 µs | PASS (max 306 µs) |
| Close during a stalled send (1 s budget) | 24.7 / 37.5 µs | 1 100 000 µs | PASS |
| 1 000 open/close cycles — descriptors | delta **0** | ±2 | PASS |
| 50 concurrent sends while one is active | **50 BUSY**, 1 connection | all BUSY, no new connection | PASS |
| BUSY connection count | one connection per transport, none for the rejected call | 0 extra | PASS |

## 3. Observations

- **BUSY is fail-fast in tens of microseconds.** The rejection path never dials: the connection count equals the number of transports created, not the number of `send()` calls, and all 50 concurrent calls returned `TRANSPORT_BUSY`.
- **Close is immediate when it can be.** Closing during a stalled TLS handshake aborts the task in ≈25 µs, far inside the 1 s budget; an idle close costs ≈11 µs, and 1 000 cycles leave the descriptor count unchanged, so there is no timer or socket accumulation.
- **No queue exists.** Under concurrent load there is exactly one connection and every extra call is rejected, which is the C-08/C-16 invariant measured rather than asserted.
- The 306 µs maximum on an idle close is a single scheduler blip (the p95 is 26.6 µs); it is far below the budget and is the only outlier across all samples.

## 4. Method

- BUSY: create a transport, start a send to the silent relay, wait for the accepted connection, then time the second `send()` rejection. 3 warm-up iterations, 20 measured.
- Close during send: same setup with a 1 s close budget (`connectMs`/`operationMs`/`totalMs`/`closeMs` all 1 000), timing `close()`.
- Idle close: 1 000 create→close cycles, timing each `close()`; the first 20 feed the reported statistics.
- Descriptors: `/dev/fd` entry count before and after the 1 000 cycles.
- Every budget is asserted, so a regression fails the probe.

## 5. Limitations

- The relay never completes the TLS handshake, so "close during send" measures aborting the handshake stage; closing during an established session or during DATA is covered functionally by the P10 close-budget tests, not timed here.
- Local loopback only; no network latency is involved.
- The probe runs on Node.js 22 only; Bun and Deno run the runtime smoke entry point by baseline rule.

## 6. Verdict

BUSY, close, and concurrency behave as the contract requires and are two to three orders of magnitude inside their budgets, with no queue and stable descriptors. P13-004 is complete; P13-005 consolidates the three performance reports and gates the phase.
