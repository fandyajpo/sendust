# Clean-Consumer Distribution Smoke (P14-001)

**Status:** complete — the packed artifact installs, imports and sends from a clean project
**Date:** 2026-11-16
**Environment:** macOS (Apple Silicon), Node.js 22.23.5 (nvm), npm 10.9.2, Rust 1.100.0
**Decision:** [ADR-0019](../architecture/decisions/0019-installable-typescript-package.md) makes `@sendust/ts` the installable package.

## 1. What the smoke proves

A project that has never seen the workspace installs the packed tarball, imports it, sends one
message to a loopback TLS relay, and closes. Nothing resolves from the repository source tree.

## 2. Sequence and results

| Step | Command | Result |
| --- | --- | --- |
| Build the package | `npm --workspace ./ts run build` | PASS — `dist/` emitted, `sendust.node` bundled |
| Pack contents | `npm --workspace ./ts run pack:check` | PASS — 17 files, all inside the allowlist |
| Clean install | `npm install sendust-ts-0.1.0.tgz` in a fresh project | PASS — exit 0, 1.18 MB tarball |
| ESM import | `import { createTransport, version } from "@sendust/ts"` | PASS — both resolve |
| Lifecycle | `createTransport(...)` then `await close()` | PASS — `open` → `closed` |
| Accepted send | one message to a loopback TLS relay, test CA as `tlsRootCertificate` | PASS — `accepted`, recipients `[b@example.test]`, relay `250 2.0.0 accepted` |
| Missing addon | remove `sendust.node`, then `createTransport` | PASS — `SendustConfigurationError`, code `CONFIG_NATIVE_UNAVAILABLE`, `instanceof` holds |
| BUSY | a second `send()` while one is active | PASS — `SendustBusyError`, code `TRANSPORT_BUSY`, rejected before the first settled |
| Post-close state | `state` after `close()` | PASS — `closed` |

## 3. Tarball allowlist

`dist/*.js`, `dist/*.d.ts`, `sendust.node`, `README.md`, `package.json`. The tarball contains no
`.env`, `docs/`, `tests/`, `src/`, source map, `.key` or `.pem` file. The allowlist, not
`.npmignore`, is the boundary, and `pack:check` fails on any path outside it.

## 4. Defects this smoke found and closed

| Defect | Cause | Fix |
| --- | --- | --- |
| A missing addon surfaced as a raw Node `MODULE_NOT_FOUND` whose message named `dist/loader.js` | The loader let the require failure escape | `loader.ts` maps only `MODULE_NOT_FOUND` to `CONFIG_NATIVE_UNAVAILABLE`; every other load failure is rethrown untouched. `fromNative` maps the tagged error to `SendustConfigurationError` |
| The format and lint gates scanned the generated `dist/` output | The package scripts ran `biome ... .` | The scripts name `src test scripts` explicitly |

## 5. Limits of this evidence

- One platform only: macOS arm64. The rest of the matrix is P14-003/P14-004.
- Node.js only. Bun and Deno loading are P14-004.
- ESM only. The package declares no CommonJS entry, so ADR-0012's dual-format requirement is not yet met. That is recorded as an open item for P14-002, not claimed here.
- The relay is a loopback fake, so this shows packaging and plumbing, not deliverability.

## 6. Next

P14-002 defines the final package metadata: name confirmation, the CommonJS entry, engine
ranges, and the loader resolution order across platform packages.
