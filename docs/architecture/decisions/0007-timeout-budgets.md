# ADR-0007 — Finite timeout budgets and limited stage attribution

**Status:** ACCEPTED (coarse-stage design); runtime timing validation required at P10-002  
**Date:** 2026-11-13  
**Deciders:** Architecture review (P02-005)  
**Validated by:** P01-003, P04-004, P10-002

## Context

SMTP sessions can hang at connect, TLS, greeting, AUTH, MAIL, RCPT, DATA, or the final response. Without finite budgets a consumer process can hang indefinitely; without honest stage reporting, errors can mislead operators into retrying an operation that already transmitted the body.

## Decision

- Defaults: connect 10 s, per-operation 30 s, total send 120 s, close/drain 125 s.
- Every timeout is finite and tunable within documented bounds. `0`, negative, `NaN`, and infinite values are rejected.
- A single absolute deadline governs the total send.
- `connect` bounds DNS + TCP + greeting + EHLO + implicit TLS/STARTTLS negotiation through one outer deadline. lettre's inner timeout only bounds TCP connection attempts; it does not by itself bound DNS, greeting/EHLO, or TLS upgrade.
- `operation` bounds each command/response wait.
- `total` bounds the entire send.
- `close` bounds shutdown and is greater than `total` so a maximal send can still drain.
- Report only coarse public-call boundaries: connect+greeting+EHLO+TLS; AUTH; MAIL; each RCPT; DATA command; DATA body+terminator+final reply; cleanup. Do not promise finer internal socket read/write stage attribution.

## Alternatives

1. **No timeouts, rely on OS TCP timeouts.** Unacceptable: can hang for minutes and ignores SMTP-level stalls.
2. **Only a total timeout.** Simpler, but produces poor diagnostics and cannot distinguish a connect stall from a DATA stall.
3. **Per-command timeouts with no total.** Can still exceed a caller’s overall budget across many commands.
4. **Promise exact stage attribution.** Rejected unless observable; otherwise it is fabricated precision.

## Consequences

- All hangs are bounded.
- A close during a maximal send has headroom.
- Tests can use fake time to assert boundaries without real waits.
- Documentation must not claim stage precision that research did not prove.
- Timeout after DATA carries duplicate-send risk for the caller. If the final SMTP disposition cannot be established, return `unknown` and do not automatically retry; see the error catalog.

## Evidence required

- Boundary tests at connect 10 s, operation 30 s, total 120 s, close 125 s.
- Interaction tests where a stage timeout and total timeout compete.
- The coarse stage-observability table in [lettre validation](../../research/lettre-0.11.23-validation.md).
- Validation tests rejecting non-finite and non-positive values.
