# Sendust Agent Instructions

This file is mandatory for every AI coding agent and human contributor working in this repository.

## 1. Read before changing code

Before implementation, read in this order:

1. `README.md`
2. `docs/README.md`
3. `docs/product/vision.md`, `docs/product/requirements.md`, and `docs/product/mvp-scope.md`
4. `docs/discovery/constraint-ledger.md`
5. `docs/architecture/overview.md`, `docs/architecture/system-design.md`, `docs/architecture/public-api.md`, `docs/architecture/data-flow.md`, and `docs/architecture/error-model.md`
6. Relevant accepted ADRs listed in `docs/architecture/decisions/README.md`
7. The relevant phase plan in `docs/phases/`
8. The matching task row in `docs/planning/task-registry.md`, dependencies in `docs/planning/dependency-graph.md`, and gate checks in `docs/planning/quality-gates.md`
9. Relevant engineering strategy and gate records.

The phase plan owns task-level implementation detail. Requirements own product behavior. Accepted ADRs own architecture decisions. The task registry and dependency graph index work; they must not silently redefine it. If two authoritative documents conflict, stop the affected change, report the exact conflict and paths, and do not choose whichever text is more convenient.

The complete operational contract is `docs/engineering/ai-implementation-contract.md`.

## 2. Execution capability disclosure

Some agents can inspect and edit files but cannot execute terminal commands, builds, tests, linters, formatters, or runtime probes.

If you cannot execute a command in the current environment:
- Do not claim that it ran.
- Do not claim compilation, tests, lint, formatting, runtime compatibility, or CI passed.
- Perform static review only and label it as static review.
- List the exact commands and environment required for external validation.
- Do not mark a task or gate complete when its acceptance criteria require executable evidence.
- You may mark an implementation deliverable as implemented but validation-pending only when the phase/task status model allows it and the distinction is explicit.

## 3. Implementation protocol

For each assigned task:

1. Read all task prerequisites and confirm they are complete with evidence.
2. Read every linked requirement, ADR, contract, and engineering strategy.
3. Inspect the current repository state; never trust a prior agent's completion claim without checking the files and evidence.
4. Produce a short file-level plan before editing.
5. Implement only the assigned task and its necessary supporting changes.
6. Preserve public contracts and module boundaries.
7. Update every affected source-of-truth document and all indexes that summarize it.
8. Review the complete diff for unintended changes, missing exports, broken links, stale examples, dependency-direction violations, and inconsistent terminology.
9. Report static review separately from executable validation.

Do not ask the user to resolve decisions already made in approved documentation. Do not invent a new architectural decision. For a genuine unresolved decision, record the question and propose an ADR or explicit decision update.

## 4. Non-negotiable architecture

- Exactly two Rust crates: `crates/sendust-core` and `crates/sendust-napi`.
- `sendust-core` has no Node.js, napi, or JavaScript runtime dependency.
- `sendust-napi` is a thin inbound Node-API adapter and depends on core, never the reverse.
- TypeScript owns the public facade and isolated native loader, not SMTP/MIME business policy.
- Node.js, Bun, and Deno are required MVP runtime targets; support is not claimed until exact-version, exact-platform blocking CI evidence exists.
- Rust core is a modular monolith with the module ownership defined by ADR-0016.
- SMTP via lettre is the only MVP outbound transport. SMTP protocol behavior belongs only in `transports::smtp`.
- No third Rust crate, microservice, public plugin registry, alternate transport, queue, worker, scheduler, persistence layer, connection pool, automatic retry, URL attachment fetch, or GC-dependent lifecycle behavior is allowed in MVP.
- TLS certificate and hostname verification are mandatory. No insecure bypass or plaintext fallback.
- One active send per transport; a second concurrent send fails immediately with BUSY. Close is explicit, idempotent, and bounded.

## 5. Change discipline

- Do not rewrite unrelated files or perform opportunistic refactors.
- Do not change accepted ADRs silently. Material architecture changes require a superseding ADR and synchronized contract/task/gate updates.
- Do not add a dependency without checking necessity, exact version/feature policy, license, maintenance, security, and impact on the core/runtime boundary.
- Do not add placeholders, fake success paths, `todo!`, or `unimplemented!` and then report the task complete.
- Keep Rust and TypeScript types explicit. Do not use TypeScript `any` in public contracts.
- Never place credentials, message bodies, attachment bytes, or unredacted filesystem paths in logs/errors.

## 6. Required completion report

Every task report must include:

- Task ID and exact scope.
- Prerequisites checked and evidence.
- Files created, changed, and deleted.
- Behavior and interfaces implemented.
- Requirements/ADR/module-boundary compliance.
- Static review performed and findings.
- Commands not executed, explicitly marked NOT RUN.
- External validation required, with exact commands and working directories.
- Remaining risks, blockers, and next task.

A task is not a phase gate. Never approve a gate yourself merely because code was written. Gate approval requires the evidence and reviewer record defined by `docs/planning/definition-of-done.md` and `docs/planning/quality-gates.md`.
