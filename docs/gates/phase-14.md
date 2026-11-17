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
| A clean consumer can install and import the package from local artifacts on every supported target | PASS | Four targets install the facade plus their platform package in a fresh consumer and construct a transport (`open`) |
| Native binary selection is correct; missing/wrong artifacts fail clearly | PASS | Bundled path first, then `@sendust/ts-<platform>-<arch>`; missing and corrupt addons both yield `CONFIG_NATIVE_UNAVAILABLE` with the package named and no dlopen path |
| Node.js, Bun, and Deno support is validated and honestly declared per exact platform combination | PARTIAL — see section 7 | Node.js 22.23.5 is validated end to end per target; Bun 1.4.4 and Deno 2.9.8 load the addon and pass the binding smoke per target; the TypeScript facade on Bun/Deno is not validated |
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
| Bun 1.4.4 | All four targets | ADDON VALIDATED | Binding smoke: load, lifecycle, error mapping |
| Deno 2.9.8 | All four targets | ADDON VALIDATED | Same, with `--allow-ffi --allow-read --allow-net` |
| Any runtime | `macos-x64` | NOT SUPPORTED — NOT BUILT | No Intel runner in this repository |
| Any runtime | musl, Windows arm64, big-endian, WASM/edge | NOT SUPPORTED | Absent from the matrix and from publication inputs |

## 4. Consolidated evidence

- **Clean install and load:** `scripts/verify-platform.mjs` writes the per-platform package, copies the built facade without an addon, installs both in a fresh consumer with `--install-links`, and constructs and closes a transport — on each runner, for its own artifact.
- **Behaviour:** the facade suite (59 tests) runs on each runner against that runner's addon as the bundled path: accepted, `451` rejection, lost final reply (`unknown`), BUSY, close races, limits, TLS trust and fail-closed, and attachment policy.
- **Negatives:** the installed addon is deleted (missing) and then replaced by a four-byte file (corrupt); both produce `CONFIG_NATIVE_UNAVAILABLE` with the expected text and no path leakage.
- **Runtimes:** Bun and Deno run `tests/runtime/smoke.mjs` per target; Node.js 22.23.5 is the gated runtime for the full suite.
- **Performance and QA:** the P13 budgets still pass (no distribution change touches them), and the P12 traceability and fixture audits remain the correctness evidence behind the behaviour suite.

## 5. Limitations

- macOS x64 has no binary and no claim until an Intel runner exists or a cross-build is proven loadable on the target.
- The facade's loader calls `process.getBuiltinModule`, which is proven on Node.js; Bun and Deno are validated at the addon level only.
- Bun and Deno are gated by the binding smoke, not by the facade behaviour suite; Vitest does not run inside them by baseline rule.
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
| Low | macOS x64 artifact job queued indefinitely | REMOVED rather than cross-compiling an unloadable binary; the row is NOT BUILT |

No high-severity finding is open. No contract, security, or performance regression was introduced.

## 7. Decision

**AWAITING APPROVAL.** The project owner must decide the scope of the runtime
criterion, because section 2 marks it PARTIAL for a reason the phase plan names
explicitly:

1. **Approve with the declaration in section 3** — Node.js supported per target, Bun and Deno validated at the addon level, facade-level Bun/Deno recorded as an open item for P16/P17 with R-041 and R-042.
2. **Hold the gate** until facade-level loading is validated on Bun and Deno, which means teaching or replacing the loader's `process.getBuiltinModule` dependency and adding a facade smoke per runtime per target.

Option 1 keeps the honest claim that no runtime is supported by compilation alone; option 2 delays P15–P17. The recommendation is option 1, because the addon is proven on all three runtimes and the remaining gap is the facade's resolution shim, which P16's release engineering can close without changing the public contract.

If approved, P14-001 to P14-005 become COMPLETED and PHASE-14 becomes APPROVED, making P15 eligible. A failed compile, test, checker, or runtime result on `main` reopens this gate.

## 8. Deferred and non-goals

- Deferred: macOS x64 binary, facade-level Bun/Deno loading, `optionalDependencies` manifest, registry-install validation, coverage tooling, advisory audit stage, `@types/node` for runtime test typechecking.
- Non-goals: cross-compiled binaries without a load check, postinstall downloads, best-effort targets, or any distribution step that changes the API contract.
