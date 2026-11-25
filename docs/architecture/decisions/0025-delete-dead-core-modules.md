# ADR-0025 — Delete the core's dead lifecycle and validation modules

**Status:** ACCEPTED (project owner decision 2026-11-25; internal only, no public API change)  
**Date:** 2026-11-25  
**Deciders:** Project owner, from the source audit of 1.2.1 (M-05, M-06)

## Context

Two core modules described rules that were already enforced elsewhere.

`crates/sendust-core/src/lifecycle` implemented `open -> closing -> closed`, a
one-send reservation and a bounded drain, with its own tests. The shipped state
machine lives in the Node-API binding — `AtomicU8` plus one slot in
`crates/sendust-napi/src/lib.rs` — so the core module had no non-test caller:
two descriptions of one lifecycle, and the tests described the copy nobody ran.

`crates/sendust-core/src/validation` claimed to be "the single source of truth
for error precedence". `validate_in_order` had no caller;
`OutgoingMessage::validate` runs its stages inline, in the approved order.

## Decision

Delete both modules, so each rule has one owner: the binding owns the transport
state machine, and the order `OutgoingMessage::validate` runs its stages in is
the precedence.

`SendDeadlines` stays public and moves to its real home in `config`, where it is
defined.

## Alternatives

1. **Move the phase machine into the core and drive it from the binding.** It
   restores ADR-0016's intent and gives the shipped lifecycle direct unit
   tests, but it is a refactor of the binding's ownership core with no
   user-visible gain here.
2. **Wire `validate_in_order` into the delivery path.** Precedence would become
   a function rather than an order, but the inline stages already return the same
   error for the same input, and the wiring would touch every validation path.
3. **Keep both.** Rejected: the audit's rule is delete or wire a contract,
   never keep a third copy of it.

## Consequences

- The core is 480 lines and 13 unit tests smaller, and none of those tests
  described behaviour a caller could observe.
- The lifecycle is covered where it is implemented: the binding's own tests and
  the facade's close tests.
- `docs/qa/traceability-matrix.md` C-08 and C-09 cite those tests rather than
  the deleted ones.
- `scripts/check-architecture.mjs` no longer lists either module.

## Evidence

- `cargo test --workspace --all-targets --locked`: 184 core tests, every suite
  green.
- `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo fmt --all -- --check`: clean.
- `npm run check`: architecture, links, claims and version agreement pass with
  both modules gone.
