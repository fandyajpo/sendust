# Phase 01 — Research

**Phase status:** NOT STARTED  
**Phase dependency:** Phase 00  
**Companion references:** [system design](../architecture/system-design.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md), [reliability strategy](../engineering/reliability-strategy.md)

## Objective

Resolve, with inspectable evidence, the dependency and platform questions that could invalidate the two-crate architecture. Research must decide whether the pinned lettre API exposes the required connection stages and recipient outcomes, whether the proposed rustls feature set exists, and whether Node 22/24/26 lifecycle behavior supports a non-GC-dependent close. It must not implement the product or install app code.

## Scope

In scope: pinned-source and documentation inspection, minimal reproducible probe plans, ADR evidence, feature-availability matrices, and explicit reject/defer decisions. Out of scope: custom SMTP implementation, production transport code, application scaffolding, package installation, and publication. If a capability is inaccessible, the phase records the conservative contract rather than inventing observability.

## Tasks

### P01-001 — Analyze lettre 0.11.23 async SMTP and version constraints

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P00-005
- **Description:** Inspect the observed `lettre 0.11.23` documentation and pinned source to enumerate the async SMTP transport surface, connection establishment behavior, envelope submission, TLS feature names, error types, and shutdown behavior. Record which APIs are public, which are lower-level connection APIs, and which stages are not externally observable. Confirm that a high-level transport is not assumed to expose recipient-level results.
- **Evidence criteria:** Capability matrix cites file/module/API names for each claim or labels it “not demonstrated.” It separately lists high-level transport, lower-level async connection, and error surfaces. It ends with a decision: use high-level transport, use lower-level API, or block pending P01-003. No unverified version drift is accepted.

### P01-002 — Validate rustls ring/webpki-roots feature and verification posture

- **Size:** S
- **Status:** NOT STARTED
- **Prerequisites:** P01-001
- **Description:** Determine the exact lettre 0.11.23 feature combination for rustls with the `ring` provider and `webpki-roots`, including transitive feature conflicts and platform implications. Verify the configuration path cannot disable certificate or hostname verification. Record failure modes for expired, self-signed, wrong-host, incomplete-chain, and malformed certificates.
- **Evidence criteria:** Feature matrix names the precise Cargo features and the source that proves them. A verification-posture statement confirms there is no public option to disable verification. Five certificate fixtures are mapped to expected errors. If features do not compose, an ADR proposes the smallest viable alternative without weakening verification.

### P01-003 — Determine recipient partial-failure and timeout-stage observability

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P01-001
- **Description:** Inspect the lower-level lettre async connection API for per-recipient RCPT outcomes, the point at which DATA begins, and visibility of connect, greeting, EHLO/STARTTLS, AUTH, MAIL FROM, RCPT TO, DATA, and final-response stages. Evaluate whether partial recipient acceptance can be reported honestly. If inaccessible, draft the ADR requiring rejection of the entire envelope before DATA while retaining `unknown` and `attempted` statuses; reject every per-recipient independent-message design.
- **Evidence criteria:** A stage-observability table marks each stage observable or not, with API citations. A recipient-outcome table covers all accepted, mixed, all rejected, and post-DATA ambiguity. The ADR contains one explicit decision: honest partial status, conservative pre-DATA rejection, or phase block. Mixed RCPT fixture is specified as one `250`, one `550`; post-DATA fixtures as an explicit final `451` (known rejection) and a dropped connection after the DATA terminator with no final reply (unknown).

### P01-004 — Validate Node 22/24/26, Node-API 8, and napi-managed Tokio lifecycle

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P00-005
- **Description:** Research napi-rs Node-API 8 behavior on candidate Node 22, 24, and 26: async task scheduling, buffer ownership, finalization, process shutdown, and whether a transport can be closed explicitly without relying on GC. Define a probe plan that holds an active send, forces GC, closes explicitly, and observes deadline-bounded drain. Record unsupported combinations rather than claiming compatibility.
- **Evidence criteria:** Version matrix lists Node 22/24/26 as candidate, probed, supported, or unsupported with evidence status. Lifecycle note proves why GC-triggered close is insufficient. Probe plan includes explicit close, missing close, forced GC, process exit during send, and close after deadline. Tokio 1 is recorded as napi-managed; no custom runtime ownership is introduced.

### P01-005 — Research decision gate and ADR handoff

- **Size:** S
- **Status:** NOT STARTED
- **Prerequisites:** P01-001, P01-002, P01-003, P01-004
- **Description:** Consolidate findings into a decision record that either clears architecture or blocks it. For each unresolved item, write the exact architecture constraint to adopt. Mandatory outcomes: transport API choice, TLS feature set, recipient-outcome policy, stage-introspection limits, Node support candidates, and confirmation that no custom SMTP is justified.
- **Evidence criteria:** Gate record has an ADR index with one decision per uncertainty, no “TBD” accepted downstream, and a mapping from each decision to P02 acceptance criteria. If P01-003 fails to demonstrate per-recipient status, the gate explicitly selects pre-DATA whole-envelope rejection and forbids promising partial status. Phase 02 is eligible only when every mandatory ADR is decided or has an explicit architecture-time deadline.

## Technical implementation details

Research consumes pinned source/docs only; it does not add crates or execute production code. Every claim must cite the observed version. The decision baseline is: call path `TypeScript wrapper -> napi -> core -> lettre`; `lettre 0.11.23` observed; Tokio 1 owned by napi; rustls `ring` plus `webpki-roots` proposed pending P01-002; no certificate-verification bypass; no custom SMTP unless documented APIs cannot satisfy the MVP; no pooling or retries. Timeout defaults are connect 10s, operation 30s, total 120s, close/drain 125s, all finite and tunable. Exact stage introspection must not be promised before P01-003. If recipient partial status is inaccessible, the architecture must reject the whole envelope before DATA and retain honest `unknown`/`attempted` results; per-recipient independent messages are explicitly rejected.

## Dependencies

Phase 01 requires Phase 00 approval. P01-001 and P01-004 depend on P00-005; P01-002 and P01-003 depend on P01-001; P01-005 depends on all research tasks. Phase 02 may not start until the research gate resolves or explicitly defers each mandatory question to a named architecture task.

## Risks and edge cases

- Documentation may describe a higher-level API that hides partial RCPT status; assuming otherwise would create a false contract.
- Cargo feature names may differ from marketing names or conflict with provider selection.
- Node 26 may be pre-release; it remains a candidate until lifecycle evidence exists.
- Async task completion after close can mask leaks if drain evidence only checks process exit.
- A lower-level connection API may expose more control but increase maintenance and custom-protocol risk; choosing it requires an explicit justification, not convenience.
- Timeout stages may be only indirectly inferable; tests must not claim exact-stage attribution without evidence.

## Testing requirements

Research tests are reproducible probes and fixture specifications, not committed product tests. Required evidence includes:

- Fake SMTP dialogue with mixed RCPT: `250` for the first recipient, `550` for the second, then observable behavior at DATA.
- Explicit final DATA response `451`: verify `rejected`, not `unknown`, and no retry. Separately verify missing final response after the terminator may have reached the relay maps to `unknown`.
- Delay fixtures at connect 11s, command 31s, total 121s, and close drain 126s against the proposed budgets.
- TLS fixtures: self-signed, expired, wrong hostname, incomplete chain, malformed certificate; all must fail closed with verification enabled.
- Node lifecycle probe: active send, forced GC, explicit close, no close, and process exit before/after deadline.
- Buffer probe: mutate and detach the JS Buffer immediately after call; copied memory must remain stable.

Each probe records the exact API boundary exercised and whether stage attribution is direct or inferred.

## Acceptance criteria

- Every mandatory uncertainty has an ADR decision or a named architecture deadline.
- TLS features are proven or replaced without weakening verification.
- Recipient-outcome behavior is decided using lower-level API evidence.
- Node 22/24/26 remain candidates unless lifecycle evidence supports stronger wording.
- The architecture phase receives no unstated assumption about exact stage introspection, per-recipient status, or custom SMTP.

## Deliverables

- Lettre capability and error-surface matrix.
- TLS feature and certificate-failure matrix.
- Recipient/stage observability report and ADR.
- Node lifecycle research matrix and probe plan.
- Research decision gate with ADR index.

## Completion gate

Gate `PHASE-01` is `APPROVED` only when P01-005 records resolved ADRs and the architecture phase has an unambiguous recipient policy, TLS feature set, runtime ownership model, and transport API direction. Any undecided mandatory item blocks Phase 02.

## Status

NOT STARTED. All tasks are planning records only.
