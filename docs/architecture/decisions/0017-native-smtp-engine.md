# ADR-0017 — Own the SMTP engine instead of delegating to lettre

**Status:** ACCEPTED (supersedes ADR-0002; implementation gate P06-005)  
**Date:** 2026-11-14  
**Deciders:** Project owner review of the native engine architecture audit  
**Validated by:** P06-001, P06-002, P06-003, P06-004, P06-005  
**Supersedes:** [ADR-0002](0002-lettre-over-custom-smtp.md)

## Context

Sendust already owns its domain: addresses and IDNA, header validation, the message model, MIME composition with wire accounting, hard caps, deterministic validation order, lifecycle and deadlines, the result model, the error taxonomy, and the narrow `DeliveryPort` seam.

The SMTP protocol itself was delegated to `lettre 0.11.23` behind a single adapter module. The 2026-11-14 architecture audit found that this delegation is contained (one module, one pin, no lettre type in the public API) but that it leaves the engine — the product's stated core value — outside Sendust's control, and that it blocks stage-accurate behaviour, STARTTLS proof, enhanced-status parsing and any protocol-level optimisation.

The audit is recorded at `scratchpad/sendust-native-engine-audit.md` (agent artifact) and summarised in the conversation that approved it.

## Decision

Sendust must own its email engine. Trusted infrastructure libraries remain in use for security-critical primitives, but no third-party library may remain the production SMTP engine or dictate the public API or internal architecture.

**Sendust owns:**

- SMTP command serialization and response parsing, including multiline replies, line/response caps, and enhanced status codes (RFC 3463).
- EHLO capability negotiation and STARTTLS upgrade sequencing (RFC 3207).
- SMTP AUTH PLAIN and LOGIN (RFC 4954); other mechanisms are out of MVP scope.
- The session state machine: connect, greeting, EHLO, STARTTLS, AUTH, MAIL, RCPT, DATA, QUIT, and their failure transitions.
- Dot-stuffing, the DATA terminator, and bounded I/O.
- Stage-accurate outcome and error mapping through the existing taxonomy.

**Trusted primitives remain third-party:**

- TLS: `rustls` with the `ring` provider and `webpki-roots` (ADR-0003), reached asynchronously through `tokio-rustls`.
- Network I/O and timers: Tokio.
- Transfer encoding and IDNA: the existing `base64` and `idna` pins (ADR-0010).

**Explicitly out of MVP scope:** DKIM (RFC 6376), SMTPUTF8 (RFC 6531/6532), CHUNKING/BDAT (RFC 3030), PIPELINING (RFC 2920), DSN (RFC 3461/3464), connection pooling, retries, queues, persistence and any third-party SMTP engine.

During migration, lettre was available as a **dev-dependency differential oracle**: the same fault matrix ran against both implementations until parity was proven. It was removed on PHASE-06 approval (2026-11-14) and is absent from the tree.

## Alternatives

1. **Keep lettre (ADR-0002).** Lowest risk and already proven end to end, but leaves the engine outside Sendust's ownership and permanently caps protocol-level control. Rejected by the project owner.
2. **Replace lettre with another SMTP crate.** Swaps one third-party engine for another; does not satisfy the ownership goal. Rejected.
3. **Own the protocol on trusted primitives (selected).** Highest effort and a new parser surface, but it is the only option that meets the stated objective while avoiding reimplementation of TLS, crypto or encoding.

## Consequences

- The public API, binding and TypeScript facade do not change: the port, `SmtpConfig`, `SmtpTransport` and `SmtpSession` surfaces stay identical.
- P04's validation, MIME, lifecycle, results and error work remains authoritative; only the SMTP adapter internals are replaced. The P04-005 evidence is preserved as historical proof of the previous backend.
- Phase 06 is rewritten as the native engine phase; its fault matrix must be at least as strict as the one the lettre adapter passed.
- A new security surface exists and must be reviewed at P11: reply parsing, line limits, smuggling, STARTTLS downgrade, truncation.
- `docs/product/requirements.md`, the constraint ledger (C-03), `AGENTS.md`, the dependency policy, the quality gates and the architecture contracts must be reconciled in the same change.
- `Date` and `Message-ID` generation are missing from the current serializer and belong to P07, not to this ADR.

## Conditions that would reverse this decision

- The native client cannot reach functional parity with the current fault matrix after a documented effort, or
- an unresolvable security defect appears in Sendust-owned parsing that cannot be mitigated with bounded input and review, or
- the project owner decides to prioritise MVP delivery and defer engine ownership.

Reversal requires a superseding ADR; lettre can be restored behind the unchanged `DeliveryPort`.

## Evidence required

- P06-001: codec and parser unit tests, including oversized lines, malformed and multiline replies.
- P06-002: implicit TLS and STARTTLS proof against the loopback harness and certificate fixtures.
- P06-003: AUTH and state-machine coverage, including 334 challenges and failure transitions.
- P06-004: differential fault matrix against the lettre oracle, then removal of lettre from production dependencies.
- P06-005: gate record with the full matrix, a security review of the parser, and updated policy documents.
