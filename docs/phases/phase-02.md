# Phase 02 — Architecture

**Phase status:** NOT STARTED  
**Phase dependency:** Phase 01  
**Companion references:** [system design](../architecture/system-design.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md), [reliability strategy](../engineering/reliability-strategy.md), [performance strategy](../engineering/performance-strategy.md)

## Objective

Freeze the authoritative architecture for a two-crate Rust system with a thin TypeScript wrapper: `TypeScript wrapper -> napi -> core -> lettre`. The architecture must encode exact status semantics, ownership boundaries, lifecycle, limits, timeout budgets, error taxonomy, and test seams so later phases implement rather than reinterpret.

## Scope

In scope: component boundaries, public TypeScript-facing contract shape, Rust core interfaces, napi ownership and error mapping, state machine, TLS posture, limit enforcement, observable status model, fault-injection seams, and ADR resolution. Out of scope: full repository scaffolding, production SMTP implementation, release packaging, and documentation site. No application code.

## Tasks

### P02-001 — Define crate boundaries and dependency direction

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P01-005
- **Description:** Specify exactly two crates: `crates/sendust-core/src` containing SMTP/MIME/attachment/validation/state logic and Rust unit tests without Node linkage, and `crates/sendust-napi/src` containing only Node-API adaptation. Define allowed dependency edges: `sendust-napi -> sendust-core`; no reverse edge; core must not depend on napi, Node headers, or JS runtime types. The TypeScript wrapper in `ts/src` depends on the napi package and exposes the public API. Justify isolation as compile-time prevention of Node coupling in core.
- **Evidence criteria:** Component diagram shows every edge and no cycles. Cargo boundary table names permitted crates per layer. A static check plan proves core does not link Node; a negative dependency test is specified. The path `TS -> napi -> core -> lettre` appears in the decision record.

### P02-002 — Specify the public status, result, and error contract

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P02-001, P01-003
- **Description:** Define the TypeScript-visible send result as relay acceptance, not delivery. Enumerate statuses including `accepted`, `rejected`, `unknown` after DATA, `attempted` recipients, `busy`, `timeout` by finite budget, `closed`, and validation failures. Define when the whole envelope is rejected before DATA versus unknown. Incorporate the P01-003 ADR: if per-recipient partial status is inaccessible, do not promise it; reject the entire envelope before DATA and retain honest aggregate `unknown`/`attempted`. Explicitly forbid per-recipient independent messages. Define error taxonomy for connect, TLS, authentication, validation, limit, busy, timeout, remote protocol, and closed transport.
- **Evidence criteria:** Status transition table covers accepted all, rejected all, mixed RCPT with pre-DATA rejection, post-DATA indeterminate disposition, busy, each timeout, and closed. Every error has a stable code and machine-readable category. Contract explicitly states no mailbox-delivery guarantee, no retry after unknown, and no per-recipient message splitting.

### P02-003 — Design transport lifecycle, concurrency, and timeout budgets

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P02-001
- **Description:** Specify the transport state machine `open -> closing -> closed`, with close idempotence and deadline-bounded drain. Allow at most one active send per transport; a second send fails fast with `BUSY` and is never queued. Define finite tunable defaults: connect 10s, operation 30s, total 120s, close/drain 125s. Define how budgets interact so total cannot exceed 120s and close cannot exceed 125s. State that close is explicit and must not depend on GC; AbortSignal is optional and not required for MVP. Validate exact stage attribution only where P01-003 proved it.
- **Evidence criteria:** State and timeout diagrams are consistent with P02-002. Matrix covers send while open, send while closing, send after closed, double close, close during connect/auth/DATA, and second-send BUSY. Retention/ownership of in-flight tasks is explicit. A test seam for deterministic time advancement is specified.

### P02-004 — Specify memory ownership, limits, and attachment policy

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P02-001
- **Description:** Define that all JS-owned Buffer data is copied into Rust-owned memory before any async yield; no borrowed JS memory crosses an await. File attachments are opt-in, filesystem access is disabled by default, only bounded regular files are read, symlinks/FIFOs/devices/directories are rejected, and URLs are unsupported. Define hard caps: 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw attachment bytes, 30 MiB wire message. Define ASCII local parts, IDNA domains, Unicode display names/bodies, and SMTPUTF8 as post-MVP. Define deterministic limit-check ordering and error precedence.
- **Evidence criteria:** Ownership table shows copy points for each input. Limit table includes inclusive boundary and over-limit behavior for every cap. Attachment policy lists allowed and rejected file types plus the no-URL rule. Address policy states exact encoding expectations. Error precedence prevents nondeterministic validation messages.

### P02-005 — Architecture review, ADR freeze, and implementation handoff

- **Size:** S
- **Status:** NOT STARTED
- **Prerequisites:** P02-001, P02-002, P02-003, P02-004
- **Description:** Run the architecture gate against every Phase 00/01 constraint. Freeze ADRs for crate isolation, transport API, recipient semantics, TLS, lifecycle, memory ownership, limits, and test seams. Produce a traceability matrix mapping each architecture decision to future implementation and test tasks. Confirm no custom SMTP unless P01 explicitly justified it; confirm no pooling, no automatic retries, and no GC-dependent close.
- **Evidence criteria:** Review record has no open contradiction. Traceability matrix maps all mandatory constraints to P03–P13 tasks. The gate is `APPROVED` only when repository scaffolding can proceed without design decisions. Any unresolved item is an explicit ADR rejection, not an implementation guess.

## Technical implementation details

The proposed design baseline, pending P02-005 approval, is as follows. Two crates only: `crates/sendust-core/src` and `crates/sendust-napi/src`. Core contains all protocol-independent orchestration and Rust unit tests; it does not link Node. The napi crate is a thin adapter that validates JS inputs, copies buffers, maps errors/statuses, and manages async tasks under napi’s Tokio 1 runtime. The TypeScript wrapper in `ts/src` presents the public API and owns no SMTP logic. `lettre 0.11.23` is the SMTP dependency; no custom SMTP unless research proved necessity. TLS uses rustls with `ring` and `webpki-roots` per P01-002; verification cannot be disabled. MVP disables pooling and automatic retries. One active send per transport; overlapping sends return `BUSY` immediately. Lifecycle is explicit `open -> closing -> closed`; close drains to a 125s deadline and never relies on GC. Timeouts: connect 10s, operation 30s, total 120s, close 125s, finite and tunable. JS buffers are copied before async. Attachments require opt-in filesystem access and bounded regular-file reads; no URL fetch. Caps: 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw, 30 MiB wire. Success means relay accepted; after DATA unknown is not retried. Recipient handling follows P01-003: if partial status is unavailable, reject the entire envelope before DATA and report honest aggregate statuses; never send independent per-recipient messages.

## Dependencies

Phase 02 requires Phase 01 approval. P02-001 depends on P01-005; P02-002 additionally consumes P01-003; P02-003 and P02-004 depend on P02-001; P02-005 depends on all architecture tasks. Phase 03 may start only after the frozen handoff.

## Risks and edge cases

- A “thin” napi layer can accidentally absorb business logic, making core tests less meaningful.
- Mapping a post-DATA failure to `rejected` would imply certainty the client cannot have.
- Timeout interaction can create budget overruns if operation and total are enforced independently without a shared deadline.
- Copying buffers introduces peak-memory duplication; caps must be enforced before large copies where possible.
- Attachment checks are vulnerable to TOCTOU and special files; the design must specify open-then-fstat semantics and rejection rules.
- An ambiguous state machine can allow sends after close initiation.
- Rustls provider/root features can conflict; architecture must cite the proven set.

## Testing requirements

Architecture tests are contract and model tests specified for later implementation, with concrete fixtures:

- Mixed RCPT `250`/`550` must produce the ADR-selected whole-envelope outcome, never per-recipient success.
- Explicit final DATA `451` yields `rejected`; a missing final reply after the DATA terminator may have reached the relay yields `unknown`. Neither is retried.
- Second concurrent send yields `BUSY` without dialing the relay.
- Close during each stage completes within 125s; double close is idempotent.
- Connect at 10s, operation at 30s, and total at 120s boundaries are asserted with fake time.
- Recipient count 100 passes, 101 fails validation before connect.
- Attachment 10 MiB passes, 10 MiB + 1 fails; aggregate 20 MiB passes, 20 MiB + 1 fails; wire 30 MiB passes, 30 MiB + 1 fails.
- Buffer mutation/detachment after call does not alter sent bytes.
- Symlink, FIFO, device, directory, and missing paths are rejected with filesystem access enabled only when opted in; without opt-in all file access fails.
- CRLF injection in display name, local part, domain, and header-like values is rejected or safely encoded.
- IDNA domain and Unicode display/body cases pass; non-ASCII local part is rejected in MVP.

The architecture must name the fake SMTP/TLS fixture seams needed for each test.

## Acceptance criteria

- Two-crate isolation and acyclic dependency direction are unambiguous.
- Status contract is relay-accepted, with honest unknown/attempted handling and no per-recipient split.
- Lifecycle, BUSY, close, and timeout budgets are complete and non-overlapping in interpretation.
- Memory-copy, attachment, address, and limit policies are deterministic.
- Every mandatory constraint maps to a later task and concrete test.

## Deliverables

- Frozen architecture decision record with ADR index.
- Crate boundary and dependency diagram.
- Public status/error contract and state machine.
- Ownership, attachment, address, and limits specification.
- Constraint-to-task traceability matrix and test-seam catalog.

## Completion gate

Gate `PHASE-02` is `APPROVED` when P02-005 confirms the frozen architecture, no contradictory status semantics, no unproven partial-recipient promise, and a complete handoff for repository and Rust core work. The gate does not authorize application implementation beyond what later phases explicitly start.

## Status

NOT STARTED. No architecture artifacts have been created.
