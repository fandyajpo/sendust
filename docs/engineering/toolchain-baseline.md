# Sendust — Toolchain and Runtime Baseline

**Baseline date:** 2026-11-13  
**Status:** selected planning baseline; exact versions must be copied into lockfiles and CI at P03.  
**Authority:** this file is the single source of truth for implementation-tool and runtime version pins. Other documents must link here rather than maintain independent version lists.

## 1. Rust toolchain

| Item | Exact baseline | Rule |
| --- | --- | --- |
| Rust toolchain | `1.100.0` stable | Pin in root `rust-toolchain.toml`; no floating `stable` in CI |
| Rust minimum supported version | `1.88.0` | Compatibility floor imposed by pinned napi-rs 3.16.2; build toolchain is newer |
| Components | `rustfmt`, `clippy` | Install through rustup toolchain configuration |
| Workspace edition | Rust 2024 | Use consistently in both crates |

Rust 1.100.0 was the stable release on the baseline date. Source: https://blog.rust-lang.org/2026/11/12/Rust-1.100.0/

## 2. Required JavaScript runtime versions

These are exact CI candidates, not support claims. A runtime becomes supported only for an exact runtime × OS × CPU × ABI/libc combination after the complete Sendust blocking suite passes.

| Runtime | Exact CI candidate | Channel/status on baseline date | Required mode |
| --- | --- | --- | --- |
| Node.js | `22.23.5` | Maintenance LTS | Blocking |
| Node.js | `24.22.3` | Maintenance LTS | Blocking |
| Node.js | `26.13.1` | Active LTS | Blocking |
| Bun | `1.4.4` | Stable | Blocking |
| Deno | `2.9.8` | Stable | Blocking |

Do not test or publish Node.js 27 alpha as an MVP target. Do not use floating `latest`, `lts`, or `stable` tags in CI. Patch updates require a reviewed baseline update and synchronized CI/docs changes.

Official version sources:
- Node.js: https://nodejs.org/en/download/ and https://nodejs.org/en/about/previous-releases
- Bun: https://bun.com/
- Deno: https://github.com/denoland/deno/releases

## 3. JavaScript development toolchain

| Tool | Exact baseline | Responsibility |
| --- | --- | --- |
| Package manager | npm `12.2.2` | Lockfile and workspace installs; no pnpm/yarn/Bun lockfile |
| TypeScript | `7.0.2` | Strict static type checking and declaration validation |
| Biome | `2.6.7` | TypeScript/JavaScript formatting and linting |
| Vitest | `5.1.0` | TypeScript unit and contract tests under Node.js |
| TypeScript module mode | ESM source | Public ESM/CJS packaging is handled by the approved build/export plan |

These are exact development-tool pins for the baseline, not consumer runtime dependencies. Install exact versions without caret or tilde ranges and commit one npm lockfile. Runtime compatibility tests must not depend on Vitest running inside Bun or Deno; use a small runtime-neutral smoke entry point executed directly by each runtime.

Official version sources:
- npm: https://docs.npmjs.com/cli/using-npm/changelog/
- TypeScript: https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/
- Biome: https://github.com/biomejs/biome/releases
- Vitest: https://vitest.dev/blog/vitest-5-1

## 4. Canonical command matrix

All commands below run from the repository root unless the row states otherwise. They are command contracts; they are **NOT RUN** until an execution-capable environment runs them against the implemented workspace.

| Concern | Canonical command | Expected result |
| --- | --- | --- |
| Rust formatting | `cargo fmt --all -- --check` | Exit 0; no formatting diff |
| Rust lint | `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0; no warning |
| Core-only unit tests | `cargo test -p sendust-core --all-targets` | Exit 0 without Node headers/runtime or napi crate |
| Native binding compile | `cargo check -p sendust-napi --all-targets` | Exit 0 for the host target |
| Workspace Rust tests | `cargo test --workspace --all-targets` | Exit 0 |
| Crate/dependency audit | `cargo metadata --format-version 1 --no-deps` and the P03 architecture guard | Exactly two Rust packages; dependency direction is valid |
| JavaScript install | `npm ci` | Reproducible install from the committed root lockfile |
| TypeScript format/lint | `npm run lint` | Biome exits 0 |
| TypeScript typecheck | `npm run typecheck` | TypeScript exits 0 with strict settings |
| TypeScript unit/contract tests | `npm run test` | Vitest exits 0 |
| Node runtime smoke | `node tests/runtime/smoke.mjs` | Native addon loads; fake-relay send and close pass |
| Bun runtime smoke | `bun run tests/runtime/smoke.mjs` | Same observable contract |
| Deno runtime smoke | `deno run --allow-ffi --allow-read tests/runtime/smoke.mjs` | Same observable contract using local `node_modules` |

P03-003 must create the scripts and architecture guard that make these commands real. If a command cannot be implemented as written, P03-003 must propose a precise replacement and update this file, phase plan, and CI contract in the same change. Never silently substitute commands.

## 5. CI and reproducibility rules

- CI uses the exact Rust, Node.js, Bun, Deno, npm, Biome, TypeScript, and Vitest versions above.
- Native runtime jobs use a matrix generated from this file; no unpinned latest channel.
- One committed `package-lock.json` is authoritative. Do not commit `pnpm-lock.yaml`, `yarn.lock`, or `bun.lockb`.
- Rust dependencies use exact versions for architecture-critical crates and a committed `Cargo.lock`.
- A toolchain version update requires a reviewed PR, compatibility results, and updates to this baseline and dependent docs.
- Runtime version pins do not imply support. Only blocking Sendust CI evidence can promote a runtime/platform tuple to supported.

## 6. Platform values intentionally not guessed

The OS/CPU candidate list is defined by ADR-0011 and ADR-0015. Minimum glibc, minimum macOS deployment version, Windows MSVC toolset, runner image versions, and cross-compilation container digests must be frozen by P14-003 from actual supported build images. They must not be guessed in product documentation or claimed before clean-install tests.
