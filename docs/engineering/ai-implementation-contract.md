# AI Implementation and Documentation Contract

**Status:** Normative repository-wide contributor contract.  
**Applies to:** Human contributors, AI coding agents, implementation-only agents, and reviewers.  
**Architecture authority:** Product requirements and accepted ADRs.  
**Task authority:** The relevant phase plan.  
**Evidence authority:** Definition of Done and quality-gate records.

## 1. Purpose

This document makes Sendust implementable by coding agents that may have repository read/write access but no terminal, build, test, lint, format, or runtime-execution capability. It also defines how all project documents must remain synchronized.

This contract does not replace product requirements, architecture documents, ADRs, phase plans, or quality gates. It defines how to interpret and maintain them.

## 2. Canonical authority and conflict resolution

Use this precedence:

1. Product behavior and scope: `docs/product/requirements.md`, then `docs/product/mvp-scope.md` and `docs/product/vision.md`.
2. Approved architecture: accepted ADRs, with later accepted superseding ADRs taking precedence over earlier ones.
3. Architecture contracts: `docs/architecture/overview.md`, `system-design.md`, `public-api.md`, `data-flow.md`, and `error-model.md`, interpreted consistently with requirements and ADRs.
4. Implementation detail: the task section in `docs/phases/phase-XX.md`.
5. Execution ordering: `docs/planning/task-registry.md` and `dependency-graph.md`.
6. Gate/evidence requirements: `docs/planning/quality-gates.md` and `definition-of-done.md`.
7. Summaries and navigation: README files, phase index, roadmap, matrices, and engineering strategies.

A lower-level document may add detail but cannot override a higher-level decision. A conflict between equal or higher authorities is a blocker: identify both passages, avoid implementing the disputed behavior, and propose a tracked correction. Never silently resolve an architecture conflict in code.

## 3. Required reading for every task

Read `AGENTS.md` and then the relevant requirements, constraint rows, architecture contracts, accepted ADRs, phase task, prerequisites, engineering strategy, quality gate, and existing implementation. Read linked documents, not only their titles.

Before editing, verify:
- The task exists exactly once in the task registry and phase plan.
- All prerequisites exist and are complete with evidence.
- The task is not blocked by an unresolved decision.
- The target files do not already implement the requested behavior.
- The requested change does not duplicate or contradict existing code.

## 4. Task specification standard

Every implementation task must specify, directly or by precise links:

1. Objective and non-goals.
2. Prerequisite task IDs.
3. Requirement and constraint IDs.
4. Applicable ADRs and architecture modules.
5. Exact files/directories to create or modify, or an explicit rule for discovering them.
6. Public/internal types, interfaces, function responsibilities, and ownership.
7. Required behavior and deterministic edge cases.
8. Dependency and feature constraints.
9. Tests to add, including positive, negative, boundary, and fault cases.
10. Acceptance criteria stated as observable conditions.
11. Evidence artifact and owning gate.
12. External validation commands and environment, where execution is required.

If a detail is intentionally deferred, name the decision owner, the task that resolves it, and the implementation behavior allowed until then. “Use best judgment” is not a substitute for an API or security decision.

## 5. Implementation workflow

For each task:

### 5.1 Inspect
Read the complete task and all linked sources. Inspect current source, manifests, tests, and history where available. Identify existing implementation and avoid duplicate work.

### 5.2 Plan
List files to create/change, module ownership, interfaces, dependency changes, tests, documentation updates, and risks. Keep the plan scoped to the task.

### 5.3 Implement
Implement the complete assigned behavior. Follow the module ownership and dependency direction. Avoid speculative abstractions and unrelated refactors. Do not add a dependency unless its need and policy compliance are recorded.

### 5.4 Static review
When execution is unavailable, manually inspect:
- Syntax and delimiters.
- Imports, exports, visibility, and module declarations.
- Type compatibility and trait bounds.
- Rust ownership, borrowing, lifetimes, Send/Sync requirements, and error propagation.
- TypeScript strictness, generics, Promise contracts, and runtime-neutral assumptions.
- Feature flags and dependency direction.
- API/serialization parity across Rust, N-API, and TypeScript.
- Security boundaries, redaction, resource caps, timeout semantics, and cleanup.
- Tests added and whether they actually assert the requirement.
- Internal links, task IDs, gate IDs, and documentation references.

Static review is not compilation and cannot prove runtime behavior.

### 5.5 Synchronize documentation
When a source-of-truth changes, update all affected dependents in the same change:
- Requirement change → constraint ledger, feature matrix, architecture/ADR as needed, phase tasks, test traceability, gates, README summaries.
- ADR change → ADR index, architecture contracts, constraint ledger, phase tasks, dependency graph, quality gates, risk register, and relevant gate record.
- Task/dependency change → phase plan, task registry, phase index/roadmap if summarized, dependency graph, quality gates, and milestone references.
- API change → public API contract, error model, data flow, TypeScript examples, tests, feature matrix, and documentation.
- Module change → ADR-0016, system design, P03/P04/P06 task details, architecture checks, and P12 audit.
- Gate change → quality-gate index, gate checklist, task registry, phase plan, definition of done, and gate record.

Do not update summaries without updating the authoritative source. Do not leave stale contradictory text elsewhere.

### 5.6 Report
List implementation facts, static review, commands not run, external validation, and blockers separately.

## 6. Execution capability and evidence levels

Use exactly these labels:

- **IMPLEMENTED:** source/config/documentation changes are present in the inspected repository state.
- **STATICALLY REVIEWED:** manually inspected; no execution implied.
- **EXECUTED PASS:** command or test actually ran and returned success; include exact command, environment, commit, and log/evidence.
- **EXECUTED FAIL:** command ran and failed; include failure evidence.
- **NOT RUN:** execution was unavailable or intentionally not performed.
- **BLOCKED:** a prerequisite, unresolved decision, or failure prevents safe progress.

Never write “tests pass”, “build succeeds”, “compiles”, “lint clean”, “runtime supported”, or “gate approved” based only on source inspection.

An implementation-only agent may implement code and tests and mark them IMPLEMENTED/STATICALLY REVIEWED, but must leave execution-dependent acceptance criteria pending. It must not mark a gate APPROVED. A separate execution-capable CI or reviewer must produce EXECUTED PASS evidence.

## 7. Required validation command records

Each phase must list exact commands, working directory, required tool versions, and expected result. Do not guess commands or tool flags. If the exact command depends on a manifest/tool choice not yet approved, the owning task must resolve and document it before the command becomes canonical.

The repository must provide CI for:
- Rust formatting and lint.
- Rust core tests without Node linkage.
- N-API crate compilation and binding tests.
- TypeScript typecheck, lint, and tests.
- Architecture/dependency-boundary checks.
- Fake SMTP/TLS tests with loopback-only networking.
- Exact Node.js/Bun/Deno runtime matrix.
- Platform-specific native artifact build/load tests.
- Security and release evidence.

## 8. Status and gate rules

- Writing code is not the same as validating code.
- A task with executable acceptance criteria cannot be COMPLETED without the required execution evidence.
- A phase gate cannot be APPROVED by the same agent that merely authored the implementation unless the gate's reviewer/evidence rules are satisfied.
- Runtime/platform support cannot be claimed until exact runtime × OS × CPU × ABI/libc combinations pass blocking CI.
- Architecture amendment approval is design approval only, not implementation approval.
- A failed or unexecuted required check remains pending; do not convert it to PASS or N/A for convenience.

## 9. Required task report

Use this structure:

```text
Task: Pxx-00y — <title>
Status: IMPLEMENTED / STATICALLY REVIEWED / NOT RUN / BLOCKED

Prerequisites:
- <task id>: <evidence link and status>

Files:
- <path>: <change and reason>

Implementation:
- <behavior/interface delivered>

Traceability:
- Requirements: <IDs>
- Constraints: <IDs>
- ADRs: <IDs>
- Modules: <owners>

Static review:
- <checks performed>
- <findings or none>

Execution:
- Tests: NOT RUN | EXECUTED PASS | EXECUTED FAIL
- Build: NOT RUN | EXECUTED PASS | EXECUTED FAIL
- Lint/format: NOT RUN | EXECUTED PASS | EXECUTED FAIL
- Runtime/platform: NOT RUN | EXECUTED PASS | EXECUTED FAIL

External validation:
- `<exact command>` — run from `<directory>`; expected evidence: <artifact>

Remaining risks/blockers:
- <items>

Next eligible task:
- <task id and reason>
```

## 10. Documentation consistency gate

Before merging a documentation change, check:
- One canonical definition per requirement, decision, task, and gate.
- All referenced IDs exist and are unique.
- Counts and status summaries match the underlying records.
- Every internal Markdown link resolves.
- Phase dependencies agree in phase plans, task registry, index, and graph.
- Every gate in the index has a detailed checklist and an owning task/record.
- Architecture diagrams and directory trees reflect the latest accepted ADR.
- Product scope, feature matrix, API, error model, tests, and examples do not contradict each other.
- Version and platform claims identify exact status: candidate, pinned, tested, supported, or deferred.
- No implementation or validation is claimed without repository/evidence support.

The documentation audit itself must report the commit SHA and exact paths inspected. A manual audit must be labeled manual; automated link/ID checks require actual execution evidence.
