# ADR-0001 — Two crates: Node-free core plus napi adapter

**Status:** ACCEPTED (planning)  
**Date:** 2026-11-13  
**Deciders:** Architecture review (P02-005)  
**Validated by:** P02-001, P03-001, P04-005, P05-005

## Context

Sendust needs Rust domain logic that is testable without Node.js, plus a Node-API binding. The common failure mode in native addons is that protocol and business logic migrates into the binding, making it untestable outside a Node process and coupling it to a runtime.

## Decision

The workspace contains exactly two crates:

- `crates/sendust-core` — all validation, message model, MIME, attachment policy, transport state machine, SMTP orchestration, result/error taxonomy. It must not depend on `napi`, Node headers, or JavaScript runtime types.
- `crates/sendust-napi` — only Node-API adaptation: input conversion, owned-memory copying, async task ownership, error/result mapping, exported class/functions.

Dependency direction is strictly `sendust-napi -> sendust-core`. The TypeScript wrapper depends on the generated binding, never the reverse.

## Alternatives

1. **Single crate with `#[cfg]`-gated napi modules.** Fewer manifests, but Node dependencies and build settings leak into core, and it is easy to accidentally import `napi` from domain code.
2. **Three crates (core, mime, smtp).** Would provide finer reuse, but there is no independent consumer in the MVP; it adds versioning and visibility overhead without a concrete requirement.
3. **TypeScript-only implementation.** Removes native complexity but abandons the product premise.

## Consequences

- Core can run `cargo test` with no Node toolchain and no addon build.
- The binding stays small and reviewable.
- A negative dependency test (for example, a CI check that `sendust-core`'s dependency tree lacks `napi`) becomes a quality gate.
- Cross-crate changes require coordinated versioning inside the workspace; workspace inheritance keeps this cheap.
- If a future second transport needs core, extraction is possible without redesign.

## Evidence required

- `Cargo.toml` workspace membership shows exactly two crates.
- `cargo tree -p sendust-core` contains no `napi` or Node-related crate.
- Core tests run in CI without building the addon.
