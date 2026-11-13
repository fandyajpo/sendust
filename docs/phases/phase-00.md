# Phase 00 — Discovery

**Phase status:** NOT STARTED  
**Phase dependency:** None  
**Companion references:** [system design](../architecture/system-design.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md), [reliability strategy](../engineering/reliability-strategy.md)

## Objective

Establish the authoritative MVP problem statement, delivery semantics, constraint ledger, and open validation questions before any research, architecture, or implementation work begins. Discovery must make it impossible for later phases to silently reinterpret “send success,” recipient failures, transport lifecycle, limits, or platform support.

## Scope

In scope: intended users and jobs, relay-accepted semantics, MVP exclusions, all stated hard constraints, candidate platform provenance, threat/operational boundary, and a phase-to-constraint traceability matrix. Out of scope: API design, crate scaffolding, dependency installation, SMTP probing, and implementation. No application code is produced in this phase.

## Tasks

### P00-001 — Capture MVP problem statement, personas, and non-goals

- **Size:** S
- **Status:** NOT STARTED
- **Prerequisites:** None
- **Description:** Write a one-page discovery brief that names the primary consumer (a Node service sending transactional mail through a configured SMTP relay), the minimum job to be done, and explicit non-goals. Define “send success” as relay acceptance, not mailbox delivery. Record that MVP has no pooling, no automatic retries, no per-recipient independent messages, no SMTPUTF8, no URL attachment fetching, and no requirement for AbortSignal. Distinguish display names/body Unicode from ASCII local parts and IDNA domains.
- **Evidence criteria:** Brief contains a falsifiable success sentence, at least three user scenarios, an explicit non-goals list, and a reviewer sign-off. Every excluded capability maps to either “post-MVP” or “rejected permanently.”

### P00-002 — Build the authoritative constraint ledger and open-question register

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P00-001
- **Description:** Convert every authoritative constraint into one ledger row with constraint text, rationale, enforcement phase, evidence owner, and validation task. Include the two-crate isolation (`sendust-core` without Node linkage, `sendust-napi` adapter), `TypeScript wrapper -> napi -> core -> lettre`, observed `lettre 0.11.23`, Tokio 1 managed by napi, no custom SMTP unless research proves need, one active send per transport with fail-fast `BUSY`, finite tunable timeouts (connect 10s, operation 30s, total 120s, close 125s), copied buffers before async, opt-in regular-file attachments with filesystem access disabled by default, 100-recipient default cap, 10 MiB per attachment, 20 MiB aggregate raw attachments, 30 MiB wire cap, and relay-accepted versus unknown-after-DATA semantics.
- **Evidence criteria:** Ledger has one row per constraint, no duplicate or contradictory wording, and each row cites a target phase/task. The unresolved recipient partial-failure question is explicitly assigned to P01-003 and P06-003; exact timeout stage introspection is assigned to P01-003. Unresolved items have owners and gate dates.

### P00-003 — Record platform and dependency provenance without installing or coding

- **Size:** S
- **Status:** NOT STARTED
- **Prerequisites:** P00-001
- **Description:** Create a provenance table for `lettre 0.11.23`, Tokio 1, Node-API 8, and candidate Node 22/24/26 runtimes. Record where each version was observed, whether it is pinned or merely proposed, and which claims still require validation. Do not assume a high-level SMTP transport exposes recipient-level status or exact stage timings; mark those as research questions rather than facts.
- **Evidence criteria:** Every version claim has a source artifact, tag, lockfile entry, or explicit “candidate” label. No “latest” assumptions. The table calls out that Node 22/24/26 are test candidates, not guaranteed support declarations.

### P00-004 — Define the threat, operational, and test-environment boundaries

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P00-002
- **Description:** Identify assets (SMTP credentials, message content, attachment files, process availability), trust boundaries (JS caller, napi boundary, Rust core, network relay), and operational assumptions (consumer owns relay configuration and credentials). Specify that tests use local deterministic fake SMTP/TLS fixtures, never live production relays or real secrets. Record abuse cases: TLS verification downgrade, CRLF/header injection, oversized envelopes, symlink/FIFO/device attachment reads, detached or mutated JS buffers, and unbounded close hangs.
- **Evidence criteria:** Boundary diagram or table identifies every trust transition. At least ten abuse-case cards exist, each mapped to a security/reliability/QA task. The test-environment policy explicitly forbids external SMTP calls and committed credentials.

### P00-005 — Run the discovery gate and hand off research questions

- **Size:** S
- **Status:** NOT STARTED
- **Prerequisites:** P00-001, P00-002, P00-003, P00-004
- **Description:** Review the brief, ledger, provenance, and boundaries in one gate session. Resolve contradictions; for anything unresolved, produce ADR seeds rather than vague notes. Mandatory ADR seeds include recipient partial-failure observability, timeout stage introspection, TLS feature validation, Node lifecycle validation, and whether any lower-level lettre SMTP API is required.
- **Evidence criteria:** Gate record lists each ADR seed with decision deadline and responsible phase. No mandatory constraint is unowned. The phase gate is marked `APPROVED` only when P01-001 through P01-004 can start with unambiguous inputs.

## Technical implementation details

Discovery is document-driven. The ledger is the canonical source for later acceptance tests. The following baseline must be preserved verbatim in intent:

- Exactly two Rust crates: `crates/sendust-core/src` and `crates/sendust-napi/src`; core has Rust unit tests and no Node linking.
- Runtime call path: TypeScript wrapper in `ts/src` -> napi binding -> Rust core -> `lettre`.
- `lettre 0.11.23` is observed, not an invitation to write a custom SMTP client.
- TLS proposal is rustls with `ring` and `webpki-roots`; feature validation belongs to P01-002. Certificate verification must never be disableable.
- MVP has no connection pooling and no automatic retry. A successful send means the relay accepted the message, not that a mailbox received it. After DATA, an unknown result is not retried.
- Recipient partial failure is not assumed to be observable. P01-003 and P06-003 must inspect lettre’s lower-level async connection API. If status is inaccessible, the architecture gate must explicitly adopt “reject the entire envelope before DATA” while retaining honest `unknown`/`attempted` statuses; no per-recipient independent messages.
- One active send per transport; concurrent attempts fail fast with `BUSY` and are never queued. Lifecycle is `open -> closing -> closed`; close drains bounded by a deadline and must not depend on garbage collection.
- Timeouts are finite and tunable: connect 10s, operation 30s, total 120s, close/drain 125s. Exact stage introspection is validated in P01-003.
- JS-owned Buffer data is copied before async work; no borrowed JS memory crosses the async boundary. File attachments are opt-in, bounded regular files only, with filesystem access disabled by default and no URL support.
- Default hard caps: 100 recipients; 10 MiB per attachment; 20 MiB aggregate raw attachment bytes; 30 MiB wire message. Caps are documented and hard, not advisory.
- Envelope local parts are ASCII; domains use IDNA; display names and bodies may be Unicode. SMTPUTF8 is post-MVP.
- AbortSignal is optional and not required for MVP; close plus deadlines are required.

## Dependencies

No phase prerequisites. P00-001 has no prerequisite; P00-002 and P00-003 depend on the statement; P00-004 depends on the ledger; P00-005 depends on all four. Phase 01 may not begin until P00-005 passes.

## Risks and edge cases

- Misreading “accepted” as “delivered” would corrupt API contracts and tests.
- Treating candidate Node versions as supported before lifecycle testing would create false compatibility claims.
- Hiding the recipient partial-failure uncertainty would force unsafe promises later.
- Omitting a constraint from the ledger would let implementation or documentation omit it.
- Assuming TLS feature names instead of validating them could produce an unbuildable dependency set.
- Requiring AbortSignal for MVP could expand scope without improving bounded shutdown.

## Testing requirements

Discovery testing is evidence review and scenario coverage, not executable product tests. Required walkthroughs use these concrete fixture cards:

1. Mixed RCPT responses: one recipient accepted, one `550`; expected policy must state whether the envelope is rejected before DATA or unknown, never per-recipient delivery.
2. `451` after final DATA: expected result is `unknown`, no retry.
3. Connect delay of 11s against 10s connect timeout.
4. Command delay of 31s against 30s operation timeout.
5. Overall send delay of 121s against 120s total timeout.
6. Close/drain exceeding 125s.
7. Exactly 100 recipients accepted; 101 rejected by validation.
8. Attachments at 10 MiB and 10 MiB + 1 byte; aggregate at 20 MiB and 20 MiB + 1 byte; wire at 30 MiB and 30 MiB + 1 byte.
9. CRLF in display name or address fields.
10. Symlink, FIFO, device, directory, and missing attachment path with filesystem access disabled by default.
11. JS Buffer mutation and detachment immediately after a send call.
12. Node GC during an active send; close must not be GC-triggered.

Each card names the responsible later task. A card without a task owner fails the gate.

## Acceptance criteria

- All five discovery tasks have reviewed evidence and `NOT STARTED` becomes eligible to change only through the phase gate.
- The constraint ledger covers every authoritative constraint, including isolation, semantics, caps, lifecycle, TLS, ownership, and platform candidates.
- The recipient partial-failure and stage-introspection uncertainties are explicit and assigned to P01-003 and P06-003.
- The test-environment boundary forbids real relays and secrets.
- Research can start without inventing requirements.

## Deliverables

- Discovery brief with problem statement, personas, scenarios, and non-goals.
- Constraint ledger and open-question register.
- Platform/dependency provenance table.
- Threat and operational boundary record with abuse-case cards.
- Gate record and ADR seed list.

## Completion gate

Gate `PHASE-00` is `APPROVED` only when P00-005 evidence shows complete constraint traceability, no unowned mandatory item, and explicit handoff questions for Phase 01. Any contradiction is resolved in the ledger or elevated to an ADR seed; it is not deferred as informal knowledge.

## Status

NOT STARTED. No phase task has begun.
