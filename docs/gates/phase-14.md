# QG-14 — Distribution Phase Record

**Decision:** AWAITING APPROVAL — project owner review
**Phase:** P14 — Distribution
**Gate:** PHASE-14
**Prepared:** 2026-11-17
**Branch:** `main`
**Evidence:** [platform matrix](../qa/platform-matrix.md), [platform validation](../qa/platform-validation.md), [clean-consumer smoke](../qa/clean-consumer-smoke.md), [package metadata contract](../qa/package-metadata-contract.md), and CI on `4606cf0`.

This record is the review package for PHASE-14. The implementation was written by an
agent, so the approval in section 7 is the required human review step, not a
self-approval. Nothing here approves P15 or any later gate.

## 1. Task evidence

| Task | Status | Evidence |
| --- | --- | --- |
| P14-001 | COMPLETED | [`clean-consumer-smoke.md`](../qa/clean-consumer-smoke.md): packed tarball installs, imports, accepts one loopback send, BUSY, close, missing-addon negative. |
| P14-002 | COMPLETED | [`package-metadata-contract.md`](../qa/package-metadata-contract.md): fields, ESM and CommonJS entries, per-platform layout, loader order, tarball allowlist. |
| P14-003 | COMPLETED | [`platform-matrix.md`](../qa/platform-matrix.md): targets, artifact names, build recipe, unsupported set, per-platform packages (ADR-0020). |
| P14-004 | COMPLETED | [`platform-validation.md`](../qa/platform-validation.md): install, load, accepted send, BUSY, close, and both negative load paths per target; Bun and Deno binding smoke per target. |
| P14-005 | IN REVIEW | This record. |

## 2. Acceptance criteria

| Phase-14 criterion | Result | Evidence |
| --- | --- | --- |
| A clean consumer can install and import the package from local artifacts on every supported target | PASS | Four targets install the packed facade tarball plus their platform package in a fresh consumer and run it under Node.js (ESM and CommonJS), Bun, and Deno |
| Native binary selection is correct; missing/wrong artifacts fail clearly | PASS | Bundled path first, then `@sendust/ts-<platform>-<arch>`; a facade with no platform package, a missing addon, and a corrupt addon all yield `CONFIG_NATIVE_UNAVAILABLE` with the package named and no dlopen path |
| Node.js, Bun, and Deno support is validated and honestly declared per exact platform combination | PASS | Node.js 22.23.5 runs the full behaviour suite per target; Bun 1.4.4 and Deno 2.9.8 run the installed tarball, the facade smoke through the loader, and the binding smoke on every target (section 3) |
| No install-time network, secret, or feature-contract violation exists | PASS | Every check installs from local folders; no postinstall, download, or compiler; no new public surface beyond `tlsRootCertificate` (ADR-0018) |
| Early smoke and final matrix are reconciled | PASS | The early smoke's one-platform, Node-only, ESM-only notes now point at P14-004 and P14-002 rather than standing as gaps |

## 3. Support declaration

This is what Sendust may claim after this gate, and nothing more.

| Runtime | Target | Status | Basis |
| --- | --- | --- | --- |
| Node.js 22.23.5 | `linux-x64-gnu` | SUPPORTED | Full behaviour suite, clean-consumer install, both negatives |
| Node.js 22.23.5 | `linux-arm64-gnu` | SUPPORTED | Same |
| Node.js 22.23.5 | `macos-arm64` | SUPPORTED | Same, plus the local `npm run verify` run |
| Node.js 22.23.5 | `windows-x64` | SUPPORTED | Same |
| Node.js 24.22.3, 26.13.1 | Linux x64 | TESTED, NOT GATED | Lint, format, typecheck, facade suite; no native addon on another OS |
| Bun 1.4.4 | All four targets | SUPPORTED (smoke) | Binding and facade smokes: load through the loader, lifecycle, error mapping |
| Deno 2.9.8 | All four targets | SUPPORTED (smoke) | Same, with `--allow-ffi --allow-read --allow-net` |
| Any runtime | `macos-x64` | NOT SUPPORTED — NOT BUILT | No Intel runner in this repository |
| Any runtime | musl, Windows arm64, big-endian, WASM/edge | NOT SUPPORTED | Absent from the matrix and from publication inputs |

## 4. Consolidated evidence

- **Clean install and load:** `scripts/verify-platform.mjs` writes the per-platform package, packs the facade with `npm pack`, installs both in a fresh consumer, and runs the installed tarball under Node.js (ESM and CommonJS), Bun, and Deno — on each runner, for its own artifact.
- **Behaviour:** the facade suite (59 tests) runs on each runner against that runner's addon as the bundled path: accepted, `451` rejection, lost final reply (`unknown`), BUSY, close races, limits, TLS trust and fail-closed, and attachment policy.
- **Negatives:** a facade installed without its platform package, the installed addon deleted, and a four-byte addon each produce `CONFIG_NATIVE_UNAVAILABLE` with the expected text and no path leakage.
- **Runtimes:** Bun and Deno run `tests/runtime/smoke.mjs` (the addon directly) and `tests/runtime/facade.mjs` (through `src/loader.ts` and `process.getBuiltinModule`) per target; Node.js 22.23.5 is the gated runtime for the full suite.
- **Performance and QA:** the P13 budgets still pass (no distribution change touches them), and the P12 traceability and fixture audits remain the correctness evidence behind the behaviour suite.

## 5. Limitations

- macOS x64 has no binary and no claim until an Intel runner exists or a cross-build is proven loadable on the target.
- Bun and Deno are proven by the two smokes, not by the full behaviour suite: Vitest does not run inside them by baseline rule, so contract breadth on those runtimes is smoke-level, which is what section 3 declares.
- Clean-consumer installs run the packed tarball on all three runtimes, but the manifest still lists no `optionalDependencies`, so npm does not yet select the platform package for the user.
- `@sendust/ts` does not yet declare the platform packages as `optionalDependencies`; that manifest change belongs to the publish-prepare step in P16.
- The relay in every check is a loopback fake, so these runs prove packaging, loading and plumbing, not deliverability.
- Install-time paths were validated from local folders, not from a registry, because publication is P17.

## 6. Findings closed during the phase

| Severity | Finding | State |
| --- | --- | --- |
| High | The artifacts job built the facade before staging the addon, so linux-x64 and linux-arm64 failed in CI | FIXED: the artifact is staged first, and the job pins Node.js 22.23.5 |
| Medium | The package build spawned `node_modules/.bin/tsc`, which npm does not create without an extension on Windows | FIXED: the compiler is started through Node |
| Medium | `verify-platform` spawned `npm.cmd` without a shell on Windows, which Node refuses (CVE-2024-27910) | FIXED: `shell: true` on Windows |
| Medium | A wrong-architecture platform package surfaced a raw dlopen message carrying absolute paths | FIXED: the typed error keeps only the code (P14-004) |
| Low | R-04's `rejected` and `unknown` were never observed from JavaScript | FIXED: loopback relay legs for a `451` terminator and a dropped final reply |
| Medium | The clean-consumer check copied the built folder and ran only under Node.js, so the tarball, the CommonJS entry, and Bun/Deno installs were unproven | FIXED: the check installs the `npm pack` tarball and runs Node ESM, Node CommonJS, Bun, and Deno, plus a facade-only install |
| Medium | A send that raced `close()` could report `TRANSPORT_BUSY` instead of `TRANSPORT_CLOSED`, depending on when the native close was polled (seen once on linux-arm64) | FIXED: the facade commits to `closing` synchronously, so the classification is deterministic |
| Medium | The runtime evidence required the addon directly, so the facade's loader was never exercised on Bun or Deno; the owner held the gate on it | FIXED: `tests/runtime/facade.mjs` runs per target with Node.js, Bun and Deno, and the loader's `process.getBuiltinModule` assumption holds on all three |
| Low | The facade smoke's closed-port case depended on how fast an OS refuses a connection; Windows reported `TIMEOUT_CONNECT` | FIXED: the relay accepts and drops the socket and the smoke expects `TLS_HANDSHAKE`, identical on all three runtimes |
| Low | macOS x64 artifact job queued indefinitely | REMOVED rather than cross-compiling an unloadable binary; the row is NOT BUILT |

No high-severity finding is open. No contract, security, or performance regression was introduced.

## 7. Decision

**AWAITING APPROVAL.** The owner held the gate on 2026-11-17 and asked for
facade-level Bun and Deno loading before approval. That work is done: the
loader's only runtime assumption, `process.getBuiltinModule`, exists on Bun
1.4.4 and Deno 2.9.8 as well as Node.js, and `tests/runtime/facade.mjs` now
imports the built package, constructs a transport through the loader, closes it,
and checks the typed error on every runtime and target (section 3).

With that, the runtime criterion is PASS and the support declaration in
section 3 is what the project may claim. Approval makes P14-001 to P14-005
COMPLETED and PHASE-14 APPROVED, which makes P15 eligible. A failed compile,
test, checker, or runtime result on `main` reopens this gate.

Options still open to the reviewer: approve, or hold again for full behaviour
suites on Bun and Deno, which would mean a non-Vitest harness for them.

## 8. Deferred and non-goals

- Deferred: macOS x64 binary, `optionalDependencies` manifest, registry-install validation, coverage tooling, advisory audit stage, `@types/node` for runtime test typechecking, full behaviour suites on Bun and Deno.
- Non-goals: cross-compiled binaries without a load check, postinstall downloads, best-effort targets, or any distribution step that changes the API contract.
