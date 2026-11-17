# Package Metadata Contract (P14-002)

**Status:** defined — one open item (CommonJS entry)
**Date:** 2026-11-16
**Package:** `@sendust/ts`, built and packed from `ts/` ([ADR-0019](../architecture/decisions/0019-installable-typescript-package.md))

## 1. Metadata fields

| Field | Value | Rule |
| --- | --- | --- |
| `name` | `@sendust/ts` | Scoped, one package; no per-platform package names in the MVP |
| `version` | `0.1.0` | Single source: `ts/package.json`. The root workspace inherits nothing and publishes nothing |
| `description` | one line, names the bundled Node-API addon | Must not claim delivery guarantees |
| `license` | `UNLICENSED` today | Publication is blocked until the owner declares a licence; recorded as an open decision |
| `private` | `true` today | Set to `false` only when publishing is approved |
| `type` | `module` | ESM-first package |
| `main` | `./dist/index.js` | ESM entry |
| `types` | `./dist/index.d.ts` | Declarations emitted by `tsc` |
| `exports["."]` | `types` then `import` | Exactly one public subpath; no `./dist/*`, `./src/*`, `./crates/*`, or wildcard exports |
| `files` | `dist/`, `sendust.node`, `README.md` | Allowlist; `scripts/check-pack.mjs` fails on any file outside it |
| `engines.node` | `>=22.23.5` | The lowest pinned runtime CI exercises; not a promise beyond it |

## 2. Tested runtimes

| Runtime | Pin | What the package is tested with | Status |
| --- | --- | --- | --- |
| Node.js | 22.23.5 | Build, full suite, clean-consumer smoke | Blocking CI |
| Node.js | 24.22.3 | Full TypeScript suite | Blocking CI |
| Node.js | 26.13.1 | Full TypeScript suite | Blocking CI |
| Bun | 1.4.4 | Runtime-neutral smoke entry point | Blocking CI, smoke only |
| Deno | 2.9.8 | Runtime-neutral smoke entry point (`--allow-ffi --allow-read`) | Blocking CI, smoke only |

None of these is a support claim: support is the intersection of an exact runtime, OS, CPU and native ABI that passes install, load and behaviour, which P14-004 owns (C-06, C-35).

## 3. Native artifact layout

- **Name and place:** one file, `sendust.node` at the package root, next to `dist/`.
- **Producer:** `scripts/build-package.mjs` compiles `src` to `dist/` and copies the staged addon from `ts/sendust.node`.
- **No optional platform packages** in the MVP: a platform without a matching build gets a clear load error, never a silent fallback (ADR-0011's rule, retained).
- **No postinstall, no download, no compiler at install time.**

## 4. Loader resolution order

`src/loader.ts` resolves the addon in this order:

1. Return the cached binding if one was already loaded.
2. Obtain `createRequire` from `process.getBuiltinModule("node:module")`; if the runtime has no such bridge, fail with `CONFIG_NATIVE_UNAVAILABLE`.
3. `require("../sendust.node")` relative to the module URL, so the addon next to `dist/` is found wherever the package is installed.
4. If the require fails with `MODULE_NOT_FOUND`, throw the typed `SendustConfigurationError` with code `CONFIG_NATIVE_UNAVAILABLE`; any other failure (broken binary, wrong ABI) is rethrown untouched so it is not misdiagnosed.

The missing-binary path is exercised in [the clean-consumer smoke](clean-consumer-smoke.md): the installed package without `sendust.node` yields the typed error, not a raw `MODULE_NOT_FOUND` naming `dist/loader.js`.

## 5. Package contents

`npm run pack:check` packs the package and fails if any file falls outside the allowlist. The current tarball contains 17 files: `dist/*.js`, `dist/*.d.ts`, `sendust.node`, `README.md`, `package.json`.

Excluded by construction: the repository `.env`, `docs/`, `crates/`, tests, TypeScript sources, source maps and caches. A root-level `npm pack` is not a release path; ADR-0019 makes `ts/` the only packaged unit.

## 6. Open item — CommonJS entry

C-22 and ADR-0012 require the package to support both ESM `import` and CommonJS `require` with identical behaviour. **This is not satisfied yet:** the shipped build is ESM-only.

Why it is not a rename: `src/loader.ts` uses `import.meta.url`, which cannot be emitted into CommonJS. A second build needs a CJS-safe module base (for example a generated `module-context` module that exports `import.meta.url` in ESM and `pathToFileURL(__filename).href` in CJS), a `require` condition in `exports`, and a clean-consumer `require()` smoke.

Owner: P14-002. This contract is not complete until that entry ships and the CJS smoke passes.
