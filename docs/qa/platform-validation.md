# Platform Validation: Clean Install and Load per Target (P14-004)

**Status:** complete — every built target passes install, load, send, BUSY, close, and both negative load paths
**Date:** 2026-11-16
**Commit:** `4606cf0` (CI, including the Bun and Deno legs) and the local macOS arm64 runs on `4882e93`…`b1cbd5c`
**Owner:** P14-004; final matrix consolidation is P14-005

## 1. What is validated

Every target that the matrix builds is checked on its own runner, against its own artifact, at the pinned Node.js version. Nothing is cross-compiled into the evidence and no target is validated by compiling only.

| Check | Where | Proves |
| --- | --- | --- |
| Artifact build and load | `Native artifact (<target>)` job, `scripts/build-artifact.mjs` + load step | The runner produced a `.node` this OS and CPU can load, with a `Transport` export |
| Clean-consumer install | `scripts/verify-platform.mjs` | The facade plus the per-platform package installs into a fresh consumer with `npm install --install-links`, without the workspace or a compiler |
| Binary selection | same | The loader falls back from the absent bundled addon to `@sendust/ts-<platform>-<arch>` and constructs a transport (`open`) |
| Accepted send | `npm test` in `ts/` on that runner | A loopback TLS relay replies `250` and the facade resolves `accepted` |
| Final rejection / lost reply | same | A `451` terminator resolves `rejected` with its response; a dropped final reply resolves `unknown` with none (R-04) |
| BUSY | same | A second concurrent send rejects with `TRANSPORT_BUSY` and opens no connection |
| Close | `npm test` and `verify-platform` | Close is idempotent, concurrent-safe, bounded during a stalled send, and the transport ends `closed` |
| Missing binary | `verify-platform` | Deleting the installed addon yields `CONFIG_NATIVE_UNAVAILABLE` naming the expected package |
| Corrupt or wrong architecture | `verify-platform` | A file that cannot be this ABI yields `CONFIG_NATIVE_UNAVAILABLE` with the load failure code and no path |

## 2. Results

CI run on `b1cbd5c`, `Native artifact (<target>)` jobs, Node.js 22.23.5, Rust 1.100.0:

| Target | Runner | Install | Load + selection | Accepted send | BUSY | Close | Missing binary | Corrupt addon |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `linux-x64-gnu` | `ubuntu-24.04` | PASS | PASS | PASS | PASS | PASS | PASS (typed) | PASS (typed) |
| `linux-arm64-gnu` | `ubuntu-24.04-arm` | PASS | PASS | PASS | PASS | PASS | PASS (typed) | PASS (typed) |
| `macos-arm64` | `macos-15` | PASS | PASS | PASS | PASS | PASS | PASS (typed) | PASS (typed) |
| `windows-x64` | `windows-2022` | PASS | PASS | PASS | PASS | PASS | PASS (typed) | PASS (typed) |
| `macos-x64` | none available | NOT BUILT | — | — | — | — | — | — |

Local confirmation on macOS 15 / Apple M2, Node.js 22.23.5, the same source:

| Command | Result |
| --- | --- |
| `npm run verify` | `verify: OK — sendust-macos-arm64.node loads` |
| `npm run verify:platform` | `load: ok: open then closed`, `missing: typed CONFIG_NATIVE_UNAVAILABLE`, `corrupt: typed CONFIG_NATIVE_UNAVAILABLE` |
| `npm test` in `ts/` | 59 passed in 6 files (57 before the R-04 legs) |
| `cargo test --workspace --locked` | all suites pass; the six performance probes stay `ignored` as designed |

The rest of the CI run is green as well: Rust workspace, native binding, TypeScript on Node.js 22.23.5, 24.22.3 and 26.13.1, the Bun and Deno runtime-neutral smokes, and the architecture/documentation checks.

### 2.1 Runtime matrix (Bun 1.4.4, Deno 2.9.8)

Each matrix runner also runs `tests/runtime/smoke.mjs` with Bun and Deno against its own staged addon, on the commit `4606cf0`:

| Check in the smoke | Expectation |
| --- | --- |
| Construct with an empty host | `CONFIG_INVALID_RELAY` |
| Open, close twice, read state | `open` → `closed` |
| Send after close | `TRANSPORT_CLOSED` |
| Send to a closed loopback port | `PROTOCOL_CONNECTION_RESET` |

| Target | Node.js 22.23.5 | Bun 1.4.4 | Deno 2.9.8 |
| --- | --- | --- | --- |
| `linux-x64-gnu` | PASS | PASS | PASS |
| `linux-arm64-gnu` | PASS | PASS | PASS |
| `macos-arm64` | PASS | PASS | PASS |
| `windows-x64` | PASS | PASS | PASS |

Honest scope: the smoke loads the addon directly and exercises the async boundary, lifecycle, and error mapping. The TypeScript facade's loader calls `process.getBuiltinModule`, which is validated on Node.js only, so facade-level loading on Bun and Deno is **not** claimed and stays open (R-041, R-042).

## 3. Negative diagnostics

Both negatives are produced from the installed package, not from the source tree, and assert the stable code rather than the message text:

- **Missing binary.** With `@sendust/ts-<key>/sendust.node` deleted, the platform package itself resolves but its entry point is gone. The loader reports `MODULE_NOT_FOUND` for that package and throws `CONFIG_NATIVE_UNAVAILABLE` with `@sendust/ts-<key> is not installed for <platform>-<arch>`, which points the user at the package to install.
- **Corrupt or wrong architecture.** With four bytes of ELF header in place of the addon, the loader keeps the platform-package failure but appends only the error code: `@sendust/ts-<key> failed to load (ERR_DLOPEN_FAILED)`. The dlopen message is deliberately dropped because it carries absolute paths.

The same two paths are also covered without a filesystem by `ts/test/platform.test.ts`, which drives `resolveNative` with a require stand-in: missing package, wrong-architecture binary, unnamed failure code, unsupported platform (`no native build for linux-x64`), bundled-addon-first order, and the untouched rethrow of a bundled failure that is not `MODULE_NOT_FOUND`.

A bundled addon that fails for any reason other than a missing module is still rethrown untouched, so a local build problem is never misreported as a packaging fault.

## 4. Scope limits

- **macOS x64 is not built.** No Intel runner is available in this repository; the job stayed queued and was removed rather than cross-compiling an unloadable binary. There is no support claim for it, and it stays out of publication inputs.
- **Per-target behaviour runs on Node.js 22.23.5.** Node.js 24.22.3 and 26.13.1 run lint, format, typecheck and the TypeScript suite on Linux x64 only; they do not load a native addon against another OS.
- **Bun and Deno load the addon on every built target**, proven by the binding smoke (section 2.1). The TypeScript facade's native path on those runtimes is not validated, because its loader depends on `process.getBuiltinModule`; that remains open (R-041, R-042).
- **musl and Windows arm64 are unsupported.** They are absent from the matrix and from publication inputs.
- **The facade does not yet declare `optionalDependencies`.** `verify-platform` installs the facade and the platform package side by side, which is the published layout but not yet the published manifest; adding the dependency list belongs to the publish-prepare step in P16.
- **No install-time network, compiler, or postinstall step is involved.** Every check installs from local folders.

## 5. Reproduction

```text
export PATH="$HOME/.nvm/versions/node/v22.23.5/bin:$PATH"
npm run verify            # addon, package, 59 tests, artifact, per-platform checks
npm run verify:platform   # clean-consumer install and the two negative paths
(cd ts && npm test)       # facade behaviour suite against the staged addon
```

CI runs the same three steps on each matrix runner, after staging that runner's artifact as the bundled addon, so the behaviour suite and the clean-consumer check both use the target's own binary.

## 6. Verdict

P14-004 is complete. Every built target installs from local artifacts, loads its own binary through the published layout, sends to a loopback relay, rejects a concurrent send as BUSY, closes bounded, and reports both failure paths with the typed code. The native addon also loads and keeps the lifecycle on Bun and Deno on every target, while the facade-level Bun and Deno path is named as open rather than assumed. The one matrix row that cannot be built is marked unsupported, and no target claims support beyond what these runs observed. P14-005 consolidates this with the QA and performance evidence for the phase gate.
