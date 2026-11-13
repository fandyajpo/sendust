# QG-01 — Research Evidence Gate

**Decision:** APPROVED WITH IMPLEMENTATION VALIDATION GATES  
**Phase:** P01 — Research  
**Reviewed:** 2026-11-13  
**Reviewer:** Independent architecture audit (GPT-6)

| Check | Result | Evidence | Remaining implementation proof |
| --- | --- | --- | --- |
| lettre API | PASS | [lettre validation report](../research/lettre-0.11.23-validation.md) | P06-003 fake SMTP integration test |
| TLS features | PASS (source graph) | [lettre validation](../research/lettre-0.11.23-validation.md), [ADR-0003](../architecture/decisions/0003-tls-rustls-ring-webpki.md) | P03-002 Cargo compile; P06-002/P11-002 certificate fixtures |
| Recipient/timeouts | PASS (source/API) | [ADR-0005](../architecture/decisions/0005-recipient-partial-failure.md), [ADR-0007](../architecture/decisions/0007-timeout-budgets.md) | P06-003/P06-004 relay traces; P10-002 timeout tests |
| Node lifecycle | PASS (design/probe plan) | [Dependency research](../engineering/dependency-research.md), [ADR-0004](../architecture/decisions/0004-napi-async-tokio.md) | P05-003 and P14-004 runtime/GC/exit matrix |
| Decision record | PASS | [ADR index](../architecture/decisions/README.md), [architecture approval matrix](../architecture/approval-matrix.md) | Implementation gates listed in matrix |

## Source findings adopted

1. Use AsyncSmtpConnection rather than the high-level transport because public command(Rcpt::new(...)) plus Error::status() exposes each RCPT response.
2. Send one MAIL and one RCPT per recipient, then one DATA transaction for the accepted-recipient subset. Never fan out independent messages.
3. Response::is_positive() accepts both 2xx and 3xx. Sendust must validate command-specific response codes, especially DATA=354 and final DATA=2xx.
4. Explicit final 4xx/5xx is rejected. A missing/indeterminate final disposition after message transmission begins is unknown; no automatic retry.
5. lettre's connect timeout only bounds TCP attempts. Sendust must apply an outer connect deadline around DNS, connect, greeting, EHLO, and TLS negotiation.
6. Use lettre features with defaults disabled: smtp-transport, tokio1-rustls, ring, webpki-roots. This excludes lettre's default pool and native-tls features. Compilation is a P03-002 hard gate.
7. Required STARTTLS has no plaintext fallback. Implicit TLS is also supported. Certificate and hostname verification stay enabled.
8. Node 22/24/26 remain candidates; no support claim is made until blocking CI tests pass.

## Environment limitation

The audit environment has Node.js 22.16.0 but no Rust toolchain (rustc/cargo). No Cargo compilation, fake SMTP/TLS runtime probe, or native-addon lifecycle test was executed. Those remain explicit P03/P05/P06/P10/P11/P14 gates. Source/API research is sufficient to decide the architecture shape, not to claim implementation has passed.

**Decision: APPROVED.** All mandatory research decisions have a selected design and explicit downstream validation owner. No unresolved design question blocks architecture review. Implementation validation remains mandatory.