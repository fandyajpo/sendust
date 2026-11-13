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
- Node.js: https://github.com/nodejs/node/releases/tag/v22.23.5, https://github.com/nodejs/node/releases/tag/v24.22.3, and https://github.com/nodejs/node/releases/tag/v26.13.1
- Bun: https://github.com/oven-sh/bun/releases/tag/bun-v1.4.4
- Deno: https://github.com/denoland/deno/releases/tag/v2.9.8

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
- npm: https://github.com/npm/cli/releases/tag/v12.2.2
- TypeScript: https://github.com/microsoft/TypeScript/releases/tag/v7.0.2
- Biome: https://github.com/biomejs/biome/releases/tag/@biomejs/biome@2.6.7
- Vitest: https://github.com/vitest-dev/vitest/releases/tag/v5.1.0

## 4. Canonical command matrix

All commands below run from the repository root. JavaScript lint/format/typecheck/test scripts run in the `./ts` npm workspace. They are command contracts; they are **NOT RUN** until an execution-capable environment runs them against the implemented workspace.

| Concern | Canonical command | Expected result |
| --- | --- | --- |
| Rust formatting | `cargo fmt --all -- --check` | Exit 0; no formatting diff |
| Rust lint | `cargo clippy --workspace --all-targets -- -D warnings` | Exit 0; no warning |
| Core-only unit tests | `cargo test -p sendust-core --all-targets` | Exit 0 without Node headers/runtime or napi crate |
| Native binding compile | `cargo check -p sendust-napi --all-targets` | Exit 0 for the host target |
| Workspace Rust tests | `cargo test --workspace --all-targets` | Exit 0 |
| Crate/dependency audit | `cargo metadata --format-version 1 --no-deps` then `node scripts/check-architecture.mjs` | Exactly two Rust packages; dependency direction and forbidden dependencies are valid |
| Documentation structure audit | `node scripts/check-docs.mjs` | Relative links, IDs, phase/task counts, and index references are valid |
| JavaScript install | `npm ci` | Reproducible install from the committed root lockfile; installs the root package and `ts` workspace |
| TypeScript lint | `npm --workspace ./ts run lint` | Biome exits 0 |
| TypeScript formatting | `npm --workspace ./ts run format:check` | Biome exits 0 with no diff |
| TypeScript typecheck | `npm --workspace ./ts run typecheck` | TypeScript exits 0 with strict settings |
| TypeScript unit/contract tests | `npm --workspace ./ts run test` | Vitest exits 0 |
| Node runtime smoke (P05 deliverable) | `node tests/runtime/smoke.mjs` | Native addon loads; fake-relay send and close pass |
| Bun runtime smoke (P05 deliverable) | `bun run tests/runtime/smoke.mjs` | Same observable contract |
| Deno runtime smoke (P05 deliverable) | `deno run --allow-ffi --allow-read tests/runtime/smoke.mjs` | Same observable contract using local `node_modules` |

P05-001 to P05-004 deliver the native binding and the TypeScript facade; `npm run build:native` stages the addon and the binding tests run in their own CI job. The three runtime smoke entry points under `tests/runtime/` remain P05-005 and P14 deliverables and are deliberately absent from CI until then. QG-03 does not require them; P05 and P14 do.

P03-003 must create the npm workspace scripts, `scripts/check-architecture.mjs`, and `scripts/check-docs.mjs` that make these commands real. The documentation checker must validate Markdown links, duplicate/missing requirement/task/ADR/gate IDs, phase/task counts, registry-to-phase task title/prerequisite parity, gate-index coverage, and runtime/toolchain claims against this file. If a command cannot be implemented as written, P03-003 must propose a precise replacement and update this file, phase plan, and CI contract in the same change. Never silently substitute commands.

## 5. CI and reproducibility rules

- CI uses the exact Rust, Node.js, Bun, Deno, npm, Biome, TypeScript, and Vitest versions above.
- Native runtime jobs use a matrix generated from this file; no unpinned latest channel.
- One committed `package-lock.json` is authoritative. Do not commit `pnpm-lock.yaml`, `yarn.lock`, or `bun.lockb`.
- Rust dependencies use exact versions for architecture-critical crates and a committed `Cargo.lock`.
- A toolchain version update requires a reviewed PR, compatibility results, and updates to this baseline and dependent docs.
- Runtime version pins do not imply support. Only blocking Sendust CI evidence can promote a runtime/platform tuple to supported.

## 6. Platform values intentionally not guessed

The OS/CPU candidate list is defined by ADR-0011 and ADR-0015. Minimum glibc, minimum macOS deployment version, Windows MSVC toolset, runner image versions, and cross-compilation container digests must be frozen by P14-003 from actual supported build images. They must not be guessed in product documentation or claimed before clean-install tests.
