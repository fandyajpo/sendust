# Package Metadata Contract (P14-002)

**Status:** complete — ESM and CommonJS both ship
**Date:** 2026-11-16
**Package:** `@sendust/ts`, built and packed from `ts/` ([ADR-0019](../architecture/decisions/0019-installable-typescript-package.md))

## 1. Metadata fields

| Field | Value | Rule |
| --- | --- | --- |
| `name` | `@sendust/ts` | Scoped, one package; no per-platform package names in the MVP |
| `version` | `0.1.0` | Single source: `ts/package.json`. The root workspace inherits nothing and publishes nothing |
| `description` | one line, names the bundled Node-API addon | Must not claim delivery guarantees |
| `license` | `MIT OR Apache-2.0` | Dual licence; `LICENSE-MIT` and `LICENSE-APACHE` at the repository root (owner decision, 2026-11-16). Publishing still needs the owner's go-ahead |
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

- **Local build:** one `sendust.node` at the package root, next to `dist/`, produced by `scripts/build-package.mjs`. The loader tries it first, which covers the source tree and the local tarball.
- **Published layout (ADR-0020):** per-platform packages `@sendust/ts-<platform>-<arch>[-<libc>]`, each containing only its addon, declared as `optionalDependencies` on `@sendust/ts`.
- **No postinstall, no download, no compiler at install time.**

## 4. Loader resolution order

`src/loader.ts` resolves the addon in this order:

1. Return the cached binding if one was already loaded.
2. Obtain `createRequire` from `process.getBuiltinModule("node:module")`; without that bridge, fail with `CONFIG_NATIVE_UNAVAILABLE`.
3. Try the bundled `sendust.node` relative to the module (or the package directory when running from source).
4. On `MODULE_NOT_FOUND`, map `process.platform`, `process.arch`, and glibc/musl (Linux) to the optional package and require it.
5. If that package is absent or the platform is unmapped, throw the typed `SendustConfigurationError` with `CONFIG_NATIVE_UNAVAILABLE` and a message naming the expected package. Any other load failure (broken binary, wrong ABI) is rethrown untouched so it is not misdiagnosed.

The missing-binary path is exercised in [the clean-consumer smoke](clean-consumer-smoke.md): the installed package without an addon yields the typed error, not a raw `MODULE_NOT_FOUND` naming `dist/loader.js`.

## 5. Package contents

`npm run pack:check` packs the package and fails if any file falls outside the allowlist. The current tarball contains 17 files: `dist/*.js`, `dist/*.d.ts`, `sendust.node`, `README.md`, `package.json`.

Excluded by construction: the repository `.env`, `docs/`, `crates/`, tests, TypeScript sources, source maps and caches. A root-level `npm pack` is not a release path; ADR-0019 makes `ts/` the only packaged unit.

## 6. CommonJS entry

C-22 and ADR-0012 require both ESM `import` and CommonJS `require` with identical behaviour. Both ship:

- The build compiles `src` twice: ESM into `dist/`, CommonJS into `dist/cjs/`.
- The CommonJS files are emitted as `.cjs`, because the package is `"type": "module"` and Node would otherwise treat `dist/cjs/*.js` as ESM.
- One shared source file, `src/module-context.ts`, holds a placeholder that the build rewrites: `import.meta.url` in the ESM output, a `pathToFileURL(__filename)` URL in the CommonJS output. The loader therefore has no format-specific code.
- `exports["."].require` points at `dist/cjs/index.cjs` with its own declarations; `exports["."].import` points at `dist/index.js`.
- Relative `require` specifiers are rewritten to `.cjs` during the CommonJS pass, and the addon resolves one level higher (`../../sendust.node`) because the CommonJS output is one directory deeper.

Verified from the packed tarball in a clean consumer on Node 22.23.5: `require("@sendust/ts")` and `import { createTransport } from "@sendust/ts"` both construct a transport (`open`) and close it (`closed`).
