# Compatibility Review — Sendust 1.2.1

Runtime, operating system, architecture, package-distribution and compatibility evidence.
Static review only. **No runtime check was executed for this audit**; every cell below cites
either source code or the committed CI definition, and distinguishes the two.

---

## 1. How compatibility is engineered

Sendust does not compile at install time. A shipped build is:

1. one `sendust.node` per OS/arch/libc target, built **natively on a runner of that target**
   (ADR-0020 — "no cross-compilation");
2. packed as a scoped optional package `@sendust/ts-<target>`;
3. resolved at load time by `ts/src/loader.ts`, which tries the bundled addon first and then
   the platform package, and fails closed with `CONFIG_NATIVE_UNAVAILABLE` when neither
   loads (`loader.ts:111-139`).

Consequences worth stating: no `postinstall`, no compiler on the host, no `node-gyp`, and no
silent fallback. A platform without a build fails at `new Transport(...)`, not on the first
send (`transport.ts:505-512`, `loader.ts:85-94`).

Seven targets are declared (`loader.ts:45-53`):

```
linux-x64-gnu   linux-arm64-gnu   linux-x64-musl   linux-arm64-musl
macos-x64       macos-arm64       windows-x64
```

---

## 2. Evidence matrix

Legend — **A** = built *and loaded* on a native runner of that exact target in CI;
**B** = exercised through the facade on that runner; **C** = claimed but not exercised;
**D** = unsupported, fails closed.

| OS / arch | libc | Node 22.23.5 | Node 24.22.3 | Node 26.13.1 | Bun 1.4.4 | Deno 2.9.8 |
| --- | --- | --- | --- | --- | --- | --- |
| Linux x64 | gnu | **A B** (`ci.yml:225`) | **B** (`ci.yml:104`) | **B** (`ci.yml:104`) | **A B** (`ci.yml:314, 336`) | **A B** (`ci.yml:317, 339`) |
| Linux arm64 | gnu | **A B** (`ci.yml:227`) | C | C | **A B** | **A B** |
| Linux x64 | musl | **A B** (`ci.yml:351-376`) | C | C | **D** (declared, `ci-musl.sh:50-51`) | **D** (declared) |
| Linux arm64 | musl | **A B** (`ci.yml:359`) | C | C | **D** | **D** |
| macOS arm64 | — | **A B** (`ci.yml:229`) | C | C | **A B** | **A B** |
| macOS x64 | — | **A B** (`ci.yml:233`) | C | C | **A B** | **A B** |
| Windows x64 | — | **A B** (`ci.yml:235`) | C | C | **A B** | **A B** |
| Windows arm64 | — | **D** — no package; `platformPackageName` returns `undefined` (`loader.ts:59-67`) | D | D | D | D |

Notes on reading the matrix:

* The Node 24/26 columns are the `typescript` job on `ubuntu-latest` only (`ci.yml:98-131`):
  lint, format, typecheck and the Vitest suite run against a mocked native module. That
  proves the **TypeScript layer** is clean on those runtimes; it does not prove the addon
  loads there. The addon is loaded only on Node 22.23.5 in CI (`ci.yml:145`, `:249`).
* Bun and Deno are installed inside the `artifacts` matrix, so they are exercised on every
  native runner, not just Linux (`ci.yml:296-339`). The standalone `bun` and `deno` jobs
  (`ci.yml:191-215`) add a runtime-neutrality check of the shared entry point.
* The musl rows run the whole Node suite plus the behaviour harness inside the musl image,
  and explicitly skip Bun and Deno: *"Bun and Deno are not claimed on musl, so those checks
  skip themselves"* (`ci-musl.sh:49-53`). This is **declared narrowness, not an omission** —
  the honest way to scope a claim.

---

## 3. Claim-by-claim verdict

| Claim (source) | Verdict | Basis |
| --- | --- | --- |
| Node.js, Bun, Deno are required MVP targets (`README.md:14`) | **Supported**, Node 22.23.5+ / Bun 1.4.4 / Deno 2.9.8 | Per-target harnesses; runtime-neutral source (no static `node:` import) |
| Node ≥ 22.23.5 (`ts/package.json:55-57`) | **Supported** | `engines`; relies on `process.getBuiltinModule`, which is why the floor is 22.x |
| Linux, macOS, Windows x64 + arm64 | **Supported where a package exists** | Two arm64 gaps: Windows arm64 has no target; Linux arm64 has no Node 24/26 evidence |
| Alpine / musl | **Node only** | `ci-musl.sh`; Bun and Deno explicitly not claimed |
| "server platform support is limited to explicitly tested OS/CPU/ABI combinations" (`README.md:15`) | **Accurate** | The matrix above is the whole surface |
| No cross-compiled binary counts (ADR-0020) | **Accurate** | Every artifact job runs its own target; musl builds inside the image |

---

## 4. Runtime-specific behaviour

### 4.1 Node.js

* Uses `process.getBuiltinModule` to reach `node:module`, `node:url`, `node:path` without a
  static import (`loader.ts:147-176`), keeping the shared module graph runtime-neutral.
* `process.report.getReport().header.glibcVersionRuntime` distinguishes glibc from musl
  (`loader.ts:70-80`), falling back to `gnu` if `process.report` is unavailable (L-11).
* Highest evidence: Node 22.23.5 on all seven targets, including the addon load, the Vitest
  suite, the documentation sample, the forced-GC probe and the behaviour harness.

### 4.2 Bun

* Works through the same `getBuiltinModule` bridge; the loader has no Bun-specific branch.
* Exercised per target through `smoke`, `facade` and `behaviour`.
* On musl: not claimed, not run.

### 4.3 Deno

* Runs with `--allow-ffi` and `--allow-read` (`ci.yml:317, 328, 339`). The FFI permission is
  **required and grants broad host access** — a real narrowing of Deno's default sandbox that
  users must accept. Worth a sentence in user-facing docs (I-10).
* Deno's `process` shim must provide `getBuiltinModule`; the CI evidence says it does at
  2.9.8, but the loader has no fallback if it does not (`unavailable("no require bridge on
  this runtime")`).

### 4.4 Module formats

* `type: "module"` with a dual `exports` map: ESM at `dist/index.js`, CJS at
  `dist/cjs/index.cjs`, each with its own types (`ts/package.json:38-49`).
* `verbatimModuleSyntax` + `isolatedModules` + `rewriteRelativeImportExtensions` in
  `tsconfig.json` mean the `.ts` extension imports are rewritten by the build rather than
  left to the runtime.

---

## 5. Distribution integrity

| Property | Observation |
| --- | --- |
| Package contents | `dist/`, one `sendust.node`, `README.md` (`ts/package.json:50-54`) — nothing else ships |
| Platform packages | Declared only in the publish manifest, because npm needs lock entries (`ci.yml:278-281`) |
| Integrity | Per-target `SHA256SUMS` and `PLATFORM-PACKAGES.sha256` in the release-candidate bundle (`ci.yml:429-432`) |
| Publication | OIDC trusted publishing + SLSA provenance (`ci.yml:492-543`) |
| Reproducibility | The facade tarball is **not** byte-identical across operating systems; the approved hash must come from the CI `facade-hash` job (`ci.yml:440-485`), recorded in `release/<version>/approved.json` |
| Version agreement | `scripts/check-versions.mjs` gates four manifests plus the facade constant in CI (`ci.yml:89-90`) |

The reproducibility position is unusually honest: rather than claiming byte-reproducibility
across hosts, the project pins a CI-produced hash and has the publisher refuse a mismatch.

---

## 6. Incompatibilities, and what a user sees

| Situation | Behaviour | Code |
| --- | --- | --- |
| Windows arm64 | Fails at construction | `CONFIG_NATIVE_UNAVAILABLE` |
| Linux with an unknown libc | Resolves the `gnu` package; a musl host without `process.report` fails to load | `CONFIG_NATIVE_UNAVAILABLE` |
| Node < 22.23.5 | Not prevented by the loader (`engines` is advisory for some installers); a missing API surfaces as `unavailable("no require bridge")` | `CONFIG_NATIVE_UNAVAILABLE` |
| Deno without `--allow-ffi` | Fails at load | `CONFIG_NATIVE_UNAVAILABLE` (the underlying permission error is converted) |
| A platform package installed for the wrong arch | Reported by error **code only**, never the dlopen message, which carries absolute paths | `loader.ts:135-138` |
| A corrupt addon (not missing) | Rethrown untouched so it is not misdiagnosed as packaging | `loader.ts:114-122` |

Every failure is closed and typed. There is no path where a missing binary degrades into a
different behaviour.

---

## 7. Compatibility findings

* **No compatibility defect was found.** The matrix is narrower than the marketing phrase
  "Node.js, Bun, and Deno" suggests, but the README's own boundary statement ("limited to
  explicitly tested OS/CPU/ABI combinations") is accurate, and every gap fails closed.
* **L-11** — the glibc/musl heuristic has a fail-open-to-`gnu` branch that can misresolve on
  a musl host with `process.report` unavailable.
* **I-10** — Deno's `--allow-ffi` requirement narrows the sandbox and is not in the user-facing
  compatibility notes.
* **Evidence gap, not a defect:** Node 24/26 are proven for the TypeScript layer only; the
  native addon is loaded on Node 22.23.5 alone. If Node 26 changes the Node-API surface in a
  way napi 3.16.2 does not anticipate, CI would not catch it on those rows. Cheap remedy: run
  the `binding` job as a Node matrix.

---

## 8. Verdict

| Question | Answer |
| --- | --- |
| Is the claimed support verified? | **Yes, for the seven targets on Node 22.23.5, and for Bun/Deno on the five gnu/Windows/macOS runners.** |
| Is anything claimed that is not verified? | Bun and Deno on musl are *explicitly not claimed*. Node 24/26 addon loading is unverified. |
| Is anything unsupported that a user might assume? | Windows arm64 (no package); Node < 22.23.5; Deno without `--allow-ffi`. |
| Is the failure mode safe? | **Yes** — `CONFIG_NATIVE_UNAVAILABLE`, no fallback build, no silent degradation. |
