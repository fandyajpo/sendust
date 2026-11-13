# Reliability Strategy

**Applies to:** P04, P05, P06, P09, P10, P12.  
**Requirements:** C-08, C-09, C-10, C-15, C-16, C-17; NFR-004, NFR-007, NFR-009, NFR-014.  
**Related:** [ADR-0006](../architecture/decisions/0006-lifecycle-single-send.md), [ADR-0007](../architecture/decisions/0007-timeout-budgets.md), [error model](../architecture/error-model.md).

## 1. Reliability goals

1. No indefinite hangs. Every network operation is bounded.
2. No hidden concurrency. One active send per transport; overlap fails fast.
3. No ambiguous shutdown. Close is explicit, idempotent, and bounded.
4. No duplicate sends. No automatic retry, especially after DATA.
5. No leaked resources. Tasks, sockets, and file descriptors are released.
6. No false certainty. Outcomes are `accepted`, `rejected`, or `unknown`, each with a precise meaning.

## 2. Lifecycle state machine

```text
        close()                drain complete
  open ─────────► closing ─────────────────► closed
    │                │
    │ send()         │ send() -> CLOSED
    ▼                ▼
  sending          (no new send)
    │
    └── outcome -> open
```

Rules:

- `send` while `open` starts one send and moves to `sending`.
- `send` while `sending` returns `BUSY` immediately, with no dial.
- `send` while `closing` or `closed` returns `CLOSED`.
- `close` while `open` transitions to `closing`, then `closed`.
- `close` while `sending` cancels or awaits the send within the close budget.
- `close` is idempotent; concurrent calls share one completion.
- The state is observable and never depends on GC.

## 3. Timeout model

| Budget | Default | Bounds |
| --- | --- | --- |
| Connect | 10 s | DNS + TCP + greeting + EHLO + implicit TLS/required STARTTLS establishment |
| Operation | 30 s | Each SMTP command/response wait |
| Total | 120 s | Entire send |
| Close | 125 s | Shutdown drain |

- A single absolute deadline governs the send.
- The close budget is greater than the total budget, so a maximal send can still drain.
- All values must be finite and > 0.
- Timeouts report category and elapsed budget; stage only if observable.
- After DATA, a timeout is `unknown` for delivery purposes and is not retried.

## 4. Failure handling matrix

| Failure point | Outcome/error | Retry by Sendust | Resource action |
| --- | --- | --- | --- |
| Validation | Sync error | No | No network |
| Configuration | Sync error | No | No network |
| Limit | Sync error | No | No network |
| Connect refused | Connection error | No | Close socket |
| TLS failure | `TLS_*` error | No | Close socket |
| AUTH failure | `AUTH_FAILED` | No | Close session |
| MAIL rejected | `rejected` | No | Close session |
| RCPT rejected | ADR-0005 outcome | No | Close session |
| DATA rejected before body | `rejected` | No | Close session |
| Body transmission interrupted | `unknown` | No | Close session |
| Explicit final 4xx/5xx response | `rejected` | No | Close session |
| Final response missing/ambiguous after DATA terminator may have reached relay | `unknown` | No | Close session |
| Timeout before DATA | Timeout error | No | Cancel, close |
| Timeout after DATA | `unknown` | No | Cancel, close |
| Panic at boundary | `INTERNAL` | No | Release send slot |

## 5. Resource ownership

- The transport owns its state and any active session.
- The active send owns its future, copied inputs, and serialized message.
- File descriptors are closed on success, error, timeout, and cancellation.
- No global state, static caches, or cross-transport pools.
- Drop implementations must not block or panic.
- Leaks are detected by counting tasks, sockets, and descriptors across repeated sends.

## 6. Race conditions to test

| Race | Expected |
| --- | --- |
| `send` vs `send` | One succeeds; other `BUSY`; one transaction |
| `close` vs `send` | Deterministic; no send after `closed` |
| `close` vs `close` | One drain; both resolve |
| `close` during connect | Cancel within close budget |
| `close` during AUTH | Cancel within close budget |
| `close` during DATA | `rejected` only if explicit negative final reply; otherwise `unknown` when final disposition cannot be observed; no retry |
| Timeout vs close | Earlier deadline wins; state consistent |
| Send completion vs close | Result delivered; transport closes |
| GC during send | No effect on lifecycle |
| Process exit during send | No hang; no GC-dependent cleanup claim |

## 7. Idempotence and repeatability

- `close` is idempotent.
- Repeating a failed validation produces the same error code and precedence.
- Repeating a send after `closed` produces `CLOSED`.
- Repeating a send after `unknown` is the caller’s decision; Sendust does not deduplicate.
- Serialization is deterministic for identical input.

## 8. Observability

MVP has no public logging or metrics API. Internally:

- No credentials, bodies, or attachment bytes are logged.
- Diagnostics are limited to host/port, stage category, elapsed budget, and stable codes.
- Errors carry machine-readable details sufficient for operations without message parsing.

Post-MVP observability hooks are evaluated in P18.

## 9. Reliability acceptance

A phase is reliable only when:

- All race tests pass repeatedly.
- All timeout boundaries pass with injected time.
- No leaked tasks, sockets, or descriptors across the test suite.
- No retry occurs after any failure.
- State is consistent after every failure path.
- Close never hangs beyond its budget.

## 10. Reliability regression policy

- A reliability defect gets a regression test before the fix.
- A race fix requires a test that fails deterministically before the fix.
- Timeout fixes use injected time, never real sleeps.
- No reliability gate is waived for schedule reasons.
