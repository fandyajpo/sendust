# ADR-0016 — Modular Monolith and Internal Module Boundaries

**Status:** ACCEPTED as Architecture Amendment A02 (design-level only; implementation validation remains in P03/P04/P06/P07/P08/P09/P12).  
**Date:** 2026-11-13  
**Owners:** Architecture  
**Supersedes:** No prior ADR. Clarifies ADR-0001, ADR-0002, ADR-0010, and ADR-0015 without changing their approved constraints.

## Context

Sendust needs modularity so SMTP, message composition, validation, attachment handling, runtime bindings, and lifecycle logic can evolve without becoming tightly coupled. Modularity must not turn a small standalone email delivery engine into a distributed system, a plugin framework, or a collection of premature crates.

The existing architecture already has a valuable process boundary: a Node-free Rust core and a thin Node-API adapter. The next step is to make the internal module boundaries explicit while preserving that boundary and the approved two-crate limit.

## Decision

Sendust adopts a **modular monolith with ports-and-adapters boundaries inside the existing two-crate Rust workspace and the TypeScript package**.

### 1. Deployment and crate boundaries

The MVP remains one installable product with one shared native addon and exactly two Rust crates:

- `sendust-core`: runtime-independent domain and application logic plus outbound transport adapters.
- `sendust-napi`: Node-API boundary only; conversion, owned-memory snapshot, lifecycle/task bridging, and error/result mapping.
- `ts/src`: public API, public types/errors, runtime-neutral wrapper, and isolated native loader.

A module is not a crate. No third Rust crate, microservice, process boundary, or separately versioned module is introduced for MVP.

### 2. Internal core modules

The following logical modules are required. They may be Rust modules/submodules, not independently published packages.

| Module | Responsibility | Must not own |
|---|---|---|
| `config` | Relay, security, auth, timeout, limits and policy configuration; deterministic config validation | SMTP session execution |
| `address` | Mailbox parsing, normalization, recipient identity | MIME serialization |
| `message` | Outgoing message model, headers and content model | Network delivery |
| `validation` | Cross-field validation, injection rules, deterministic validation order | Runtime-specific input conversion |
| `attachments` | Attachment input policy, safe bounded reads, filename/content metadata | URL fetching or unbounded streaming |
| `mime` | MIME tree composition, encoding, serialization and wire-size accounting | SMTP connection management |
| `delivery` | Use-case orchestration: validate, compose, submit once, map delivery certainty | Protocol parsing or retry scheduling |
| `transports::smtp` | SMTP session, TLS, AUTH, envelope, RCPT/DATA and SMTP response mapping using lettre | Public TypeScript API |
| `lifecycle` | Transport state, one-send reservation, BUSY, close/drain coordination | Queueing or background scheduling |
| `errors` / `results` | Stable internal error taxonomy and relay-acceptance result model | JS class construction |

The exact file layout may change during P03/P04, but dependency direction and ownership must not.

### 3. Dependency direction

The intended dependency flow is inward-to-outward and acyclic:

```text
sendust-napi ──> sendust-core public facade
                       │
                       ├── domain: config, address, message, results, errors
                       ├── application: validation, delivery, lifecycle
                       └── outbound adapter: transports::smtp ──> lettre/Tokio/rustls
```

Domain modules must not depend on SMTP, lettre, napi, Node, or runtime-specific APIs. Application orchestration may depend on internal domain contracts and a narrow outbound delivery port. The SMTP adapter implements that port. The core public facade exposes only the types and operations required by the binding and tests; internal module details remain private by default.

No circular module dependencies. No protocol behavior duplicated in TypeScript or napi.

### 4. Transport extensibility

SMTP is the only implemented and supported MVP transport. The core may define a small internal delivery-port contract at the point where orchestration needs to submit a composed message and obtain a transport outcome. The SMTP adapter is the sole MVP implementation.

Do not add generic provider registries, dynamic plugin loading, runtime reflection, configuration-driven module discovery, or public custom-transport APIs. Additional transports such as SES, SendGrid, Mailgun, or sendmail remain POST-MVP and require separate product scope, ADR, security review, and contract tests. They must not weaken SMTP result semantics or introduce provider-specific behavior into domain modules.

### 5. Optional capabilities and feature flags

Optional capabilities must be isolated behind explicit internal interfaces and only compiled/configured when they have a real implementation and test coverage. Do not add empty extension points or speculative feature flags. DKIM, connection pooling, alternate transports, SMTPUTF8, streaming attachments, and observability integrations remain deferred according to the feature matrix. Queue/job orchestration and queue adapters remain HELD outside the roadmap.

### 6. Runtime adapters

Node.js, Bun, and Deno continue to use the same public TypeScript contract and shared Node-API addon by default under ADR-0015. Runtime-specific behavior is restricted to the native loader and narrowly scoped compatibility code. No runtime-specific SMTP, MIME, validation, or delivery policy is allowed.

### 7. Testing and modularity evidence

Each module must have tests for its own invariants. Cross-module behavior must be tested through public core/application contracts and the loopback SMTP harness. Dependency checks must prove:

- exactly two Rust crates;
- no Node/napi dependency in core;
- no lettre dependency in domain modules;
- SMTP protocol code exists only in the SMTP adapter;
- no upward or cyclic module dependency;
- no duplicate SMTP/MIME/validation policy in TypeScript or napi;
- one SMTP implementation and no unimplemented generic plugin framework.

P03 establishes structure and dependency checks. P04/P06/P07/P08 implement and validate core modules. P09 validates the public facade. P12 audits coverage and architecture boundaries.

## Consequences

**Benefits**
- Internal responsibilities can evolve independently without increasing crate/package count.
- SMTP-specific concerns remain isolated from message/domain rules.
- Future transports have a clear extension seam without promising them in MVP.
- Runtime adapters remain thin and share the same business behavior.
- Unit and integration tests can target module contracts.

**Costs**
- Requires disciplined dependency direction and module ownership.
- A port abstraction adds some indirection; it must remain minimal and justified by delivery orchestration.
- The exact module layout may need refinement as P03/P04 reveal real Rust API constraints.

## Rejected alternatives

- **One flat core module:** too much responsibility and poor test seams.
- **One crate per module:** unnecessary compile, dependency, release, and API overhead; violates the two-crate MVP boundary.
- **Microservices/distributed modules:** incompatible with a standalone native library and adds operational complexity without value.
- **Generic plugin framework:** speculative; only SMTP is in MVP.
- **Separate runtime-specific engines:** duplicates behavior and violates the shared-core/runtime-neutral policy.

## Validation status

This ADR approves the design only. It does not claim that modules, interfaces, dependency checks, or tests have been implemented. P03/P04 and downstream gates must supply executable evidence.
