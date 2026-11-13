# Phase 03 — Repository

**Phase status:** IN REVIEW  
**Phase dependency:** Phase 02, Architecture Amendment A01, and Architecture Amendment A02  
**Companion references:** [system design](../architecture/system-design.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md), [release strategy](../engineering/release-strategy.md)

## Objective

Establish the repository skeleton and engineering guardrails needed to implement the frozen two-crate architecture without accidentally coupling core to Node. This phase creates structure, manifests, quality gates, and test harness seams only; it does not implement SMTP behavior.

## Scope

In scope: workspace layout, crate manifests, `ts/src` package skeleton, test directories, formatting/linting/type-check commands, dependency policy, CI guardrails, and local fake-service harness scaffold. Out of scope: product features, SMTP dialogue logic, release packaging, and public documentation. No dependency installation is performed in planning; manifests are specified as future work.

## Tasks

### P03-001 — Create the workspace and two-crate skeleton

- **Size:** M
- **Status:** IN REVIEW
- **Prerequisites:** P02-005, A01-004, A02-003
- **Description:** Specify the workspace layout with exactly two crates at `crates/sendust-core/src` and `crates/sendust-napi/src`, plus `ts/src` and `tests/`. Define root workspace membership, the exact Rust toolchain `1.100.0` and MSRV `1.88.0` from the [toolchain baseline](../engineering/toolchain-baseline.md), shared lint/format configuration, and crate responsibilities. Core must compile without Node headers or napi dependencies; napi depends on core and the binding toolkit. Include placeholder module trees that reflect architecture boundaries without implementing product logic.
- **Evidence criteria:** Layout tree matches the proposed paths exactly and contains no third Rust crate. Core manifest has no napi/Node dependency; napi manifest depends on core. A documented check fails if a reverse dependency or extra crate appears. `cargo metadata` and package discovery are part of the acceptance evidence.

### P03-002 — Define dependency pinning, feature, and license policy

- **Size:** S
- **Status:** IN REVIEW
- **Prerequisites:** P03-001, P01-002
- **Description:** Record pinned versions and allowed feature sets for `lettre = 0.11.23` (defaults off; `smtp-transport`, `tokio1-rustls`, `ring`, `webpki-roots`), `napi = 3.16.2` (`napi8`, `async`), `napi-derive = 3.7.0`, `napi-build = 2.6.0`, Tokio 1, rustls, and test-only tooling. Require `cargo tree` to prove lettre pool/native-tls are absent. Define a policy for adding dependencies, lockfile updates, license review, duplicate-version checks, and prohibited crates (custom SMTP stacks, insecure TLS toggles, URL fetchers). No dependency is installed in this planning phase; the manifest intent and review process are documented.
- **Evidence criteria:** Dependency table lists direct versus transitive intent, feature flags, and pin strategy. Policy names approval and review steps and lists prohibited capabilities. TLS policy confirms no verification-disable feature. License inventory template exists and is required for every direct dependency.

### P03-003 — Establish formatting, linting, type-check, and unit-test commands

- **Size:** M
- **Status:** IN REVIEW
- **Prerequisites:** P03-001
- **Description:** Implement the exact command matrix in the [toolchain baseline](../engineering/toolchain-baseline.md): Rust fmt/clippy/core-only tests/binding checks; npm install; Biome lint; TypeScript strict typecheck; Vitest; and direct Node.js/Bun/Deno smoke entry points. Use the pinned exact tool versions and one root npm lockfile. Specify fail-fast ordering and CI expectations, including a guard that core commands never require a Node runtime.
- **Evidence criteria:** Every command in the toolchain baseline exists as a script or executable entry point, has a defined working directory and expected exit code, and runs in CI with exact pinned tool versions. The architecture checker rejects extra Rust crates, reverse dependencies, and runtime-specific policy in core. The documentation checker validates links, IDs, phase/task counts, registry-to-phase task parity, gate indexes, and toolchain pin references. A core-only job proves no Node linkage. TypeScript and napi checks are separate. The matrix is referenced by every later task’s evidence requirements.

### P03-004 — Scaffold deterministic fake SMTP and TLS test harness seams

- **Size:** L
- **Status:** IN REVIEW
- **Prerequisites:** P03-001
- **Description:** Define harness interfaces and fixture directories under `tests/` for scripted SMTP dialogues, controllable delays, mixed RCPT responses, post-DATA failures, TLS certificate variants, connection resets, and abrupt server closes. Harness must bind to loopback only, use ephemeral ports, require no external network, and expose deterministic synchronization so tests do not sleep arbitrarily. Include fixture naming for fault injection and a policy that production code has no test-only branches.
- **Evidence criteria:** Harness contract specifies start/stop, dialogue script, delay injection, response injection, and observed commands. Fixture catalog includes mixed RCPT, an explicit final DATA `451` rejection, a dropped connection after the DATA terminator with no final reply (`unknown`), connect stall, command stall, close stall, self-signed, expired, wrong-host, incomplete-chain, malformed cert, reset, and abrupt close. Loopback/ephemeral/no-network rules are explicit.

### P03-005 — Repository gate and baseline verification

- **Size:** S
- **Status:** IN REVIEW
- **Prerequisites:** P03-001, P03-002, P03-003, P03-004
- **Description:** Validate the skeleton against the architecture handoff: exactly two crates, correct dependency direction, no product logic, no secrets, reproducible formatting/lint/type-check/test command definitions, and a complete fake-service seam plan. Record any deferred setup as a named task input rather than hidden debt.
- **Evidence criteria:** Gate record includes layout verification, dependency-direction check, secret scan, command matrix review, and fixture catalog completeness. No application code exists beyond placeholders. The repository phase is marked `APPROVED` only when Phase 04 can add core modules without structural changes.

## Technical implementation details

The repository is a Rust workspace plus a TypeScript package. Proposed paths are fixed: `crates/sendust-core/src`, `crates/sendust-napi/src`, `ts/src`, `tests/`. Exactly two crates are allowed. `sendust-core` must not link Node or depend on napi; `sendust-napi` adapts core to Node-API 8 and Tokio 1 is managed by napi. The call path remains `TS wrapper -> napi -> core -> lettre`. Pin `lettre 0.11.23` as observed. TLS features must come from P01-002 and must not permit disabling verification. Test harnesses run locally against loopback fake SMTP/TLS only; no live relay, no committed secrets. No pooling, retries, custom SMTP, URL attachment fetching, or GC-dependent close may be introduced by scaffolding. CI guardrails must fail on a third crate, reverse dependency, or core Node linkage. Shared TypeScript wrapper code must be audited for Node-only globals; native loading stays isolated. Add pinned Bun and Deno smoke jobs and Deno local-node_modules/FFI-permission setup.

## Dependencies

Phase 03 requires Phase 02 approval, A01-004, and A02-003. P03-001 depends on P02-005, A01-004, and A02-003; P03-002 depends on P03-001 and P01-002; P03-003 and P03-004 depend on P03-001; P03-005 depends on all repository tasks. Phase 04 requires P03-005.

## Risks and edge cases

- A convenience “common” crate would violate the two-crate constraint; shared code must live in core.
- napi build tooling can inject Node dependencies into core if manifests are unstructured.
- Test harnesses that use real sleeps or external relays become flaky and unsafe.
- Feature unification can enable unwanted lettre/TLS features; pinning must be reviewed at workspace level.
- TypeScript package layout can accidentally duplicate protocol logic outside the wrapper.
- CI may pass locally but fail when core tests require Node due to implicit build scripts.

## Testing requirements

Repository validation includes structural tests and harness self-tests, all using local fixtures:

- Workspace membership test asserts exactly `sendust-core` and `sendust-napi`.
- Dependency-direction test asserts core has no napi/Node linkage.
- Command smoke definitions are dry-run reviewed; implementation later proves them.
- Harness self-test scripts a mixed RCPT fixture (`250`, `550`), explicit final DATA `451`, missing final reply after DATA terminator, 11s connect stall, 31s command stall, and 126s close stall with fake time or bounded local delays.
- TLS harness serves self-signed, expired, wrong-host, incomplete-chain, and malformed certificates.
- Secret scan fixture contains a fake credential and must be detected.
- No smoke test contacts a non-loopback address.

## Acceptance criteria

- Exactly two crates exist with acyclic dependency direction and no Node linkage in core.
- `sendust-core` has the approved modular-monolith skeleton and a narrow public facade.
- Domain module dependency rules and SMTP adapter containment are statically checkable.
- Dependency and feature policy is pinned, reviewable, and forbids insecure TLS toggles.
- Canonical quality commands are defined and separated by Node dependence.
- Fake SMTP/TLS harness seams cover all required fault fixtures without external network.
- No product behavior or application code is implemented.

## Deliverables

- Workspace and crate skeleton with manifests and module placeholders.
- Dependency, feature, license, and prohibition policy.
- Quality-command matrix and CI guardrail specification.
- Fake SMTP/TLS harness contract and fault-fixture catalog.
- Repository gate record.

## Completion gate

Gate `PHASE-03` is `APPROVED` when P03-005 proves the structure, dependency direction, command baseline, and harness seams are ready for Phase 04. Any third crate, core Node linkage, secret, or external-network test dependency blocks approval.

## Status

IN REVIEW. Every P03 task has its deliverable and evidence, and the QG-03
record is prepared for independent review. The two-crate workspace, dependency
and feature policy, quality-command scripts, and fake SMTP/TLS test seams are
implemented and verified in CI.

## Evidence log

Evidence labels follow the [AI implementation contract](../engineering/ai-implementation-contract.md).

| Task | Evidence | State |
| --- | --- | --- |
| P03-001 | Two-crate workspace, manifests, module tree, `ts/src` skeleton, `rust-toolchain.toml`; `cargo metadata --format-version 1 --no-deps` reports exactly `sendust-core` and `sendust-napi` | IMPLEMENTED, STATICALLY REVIEWED |
| P03-002 | [Dependency policy](../engineering/dependency-policy.md) plus workspace pins; `cargo tree -e features` resolves `lettre` 0.11.23 with only `smtp-transport`, `tokio1-rustls`, `ring`, and `webpki-roots`, with no `pool` or `native-tls`; a core unit test constructs the pinned `SmtpTransport` builder, so a dropped transport feature fails the build | IMPLEMENTED, STATICALLY REVIEWED |
| P03-005 | [QG-03 repository baseline record](../gates/phase-03-repository-baseline.md) covering layout, dependency direction, command matrix, fixture catalog, secret scan, and a full CI run | IN REVIEW — awaiting independent review |
| P03-003 | `scripts/check-architecture.mjs`, `scripts/check-docs.mjs`, npm workspace scripts, `.github/workflows/ci.yml`; `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test -p sendust-core --all-targets`, `cargo test --workspace --all-targets`, and the four `npm --workspace ./ts run` commands exited 0; CI now runs every Rust command with `--locked` | EXECUTED PASS in a development container (Rust 1.100.0, Node.js 24.22.x) and in Sendust CI on branch `feat/p03-repository-skeleton`: the Rust workspace, Architecture and documentation checks, TypeScript (Node.js 22.23.5, 24.22.3, 26.13.1), Bun 1.4.4, and Deno 2.9.8 jobs all passed |
| P03-004 | `crates/sendust-core/tests/support/mod.rs` (loopback-only fake SMTP: scripted per-stage replies, per-recipient RCPT, injected delays, command trace), `crates/sendust-core/tests/harness_self_test.rs` (7 passing tests), `scripts/generate-tls-fixtures.sh` (authority keys discarded, leaf keys only), committed `tests/fixtures/tls/*`, and `scripts/check-secrets.mjs` with a self-test that proves detection; contract in [the harness contract](../engineering/test-harness-contract.md) | IMPLEMENTED, EXECUTED PASS locally and in CI |

Executed evidence exists in a local toolchain and in Sendust CI on branch
`feat/p03-repository-skeleton`. Gate PHASE-03 is **IN REVIEW**, not approved:
the record at [phase-03-repository-baseline](../gates/phase-03-repository-baseline.md)
needs an independent reviewer.

`Cargo.lock` is committed and every Rust CI command runs with `--locked`.
`package-lock.json` is committed and CI installs with `npm ci`.


## Multi-runtime amendment requirements

P03-001 through P03-005 must account for ADR-0015 and ADR-0016. The repository baseline must define:
- exact pinned runtime versions for Node.js, Bun, and Deno in CI;
- modular-monolith directory structure within exactly two crates;
- documented module ownership and allowed dependency directions;
- static checks for domain isolation and SMTP adapter containment;
- blocking native-addon smoke jobs for all three runtimes;
- runtime-neutral TypeScript checks and isolated native loader;
- Deno local node_modules plus --allow-ffi test configuration;
- OS/CPU/libc artifact metadata and a runtime × platform result schema;
- no postinstall compilation or network binary download.

The P03 gate may approve scaffolding and CI design only. It must not mark any runtime/platform combination supported before P05/P14 executable evidence exists.
