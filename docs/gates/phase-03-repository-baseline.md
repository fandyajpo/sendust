# QG-03 — Repository Baseline Record

**Decision:** APPROVED — project owner review, 2026-11-13  
**Phase:** P03 — Repository  
**Gate:** [QG-03](../planning/quality-gates.md)  
**Prepared:** 2026-11-13  
**Branch:** `feat/p03-repository-skeleton`  
**Evidence:** the branch head passes all seven CI jobs (Rust workspace, Architecture and documentation checks, TypeScript on Node.js 22.23.5/24.22.3/26.13.1, Bun 1.4.4, Deno 2.9.8)

## 1. Task evidence

| Task | Status | Evidence |
| --- | --- | --- |
| P03-001 | COMPLETED | Two-crate workspace, ADR-0016 module tree, `ts/src` skeleton, pinned `rust-toolchain.toml`; `cargo metadata --no-deps --locked` reports exactly two packages |
| P03-002 | COMPLETED | [Dependency policy](../engineering/dependency-policy.md); `cargo tree -e features` shows only `smtp-transport`, `tokio1-rustls`, `ring`, `webpki-roots`; `Cargo.lock` committed and used with `--locked` |
| P03-003 | COMPLETED | `scripts/check-architecture.mjs`, `scripts/check-docs.mjs`, `scripts/check-secrets.mjs`, npm workspace scripts, root `package-lock.json`, CI matrix with pinned tool versions |
| P03-004 | COMPLETED | Loopback fake SMTP harness plus 7 self-tests, committed TLS fixtures with authority keys discarded, generator script, harness contract |
| P03-005 | COMPLETED | This record, approved by the project owner |

## 2. QG-03 checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Workspace | PASS | Two crates build; TS workspace typechecks and tests |
| Toolchain | PASS | `rust-toolchain.toml` pins 1.100.0; Node/Bun/Deno pinned in CI; `check-docs.mjs` enforces pin parity |
| Format/lint | PASS | `cargo fmt --check`, `cargo clippy -D warnings`, Biome lint/format, strict `tsc` |
| Tests | PASS | 1 core unit test, 7 harness self-tests, Vitest (no suites yet by design) |
| Dependency policy | PASS | Exact pins, `--locked`, committed lockfiles, license inventory, prohibited capabilities listed |
| Test seams | PASS | Fake SMTP starts, scripts responses, injects delays, shuts down deterministically; TLS fixtures verified by `openssl` in CI |
| No placeholders | PASS | No `todo!()`, `unimplemented!()`, or fake success paths. The empty core modules and doc-only TS files are the P03-001 skeleton deliverable and are labelled as such, not as behavior |
| Secret scan | PASS | `check-secrets.mjs` scans for PEM keys and token shapes, exempts only TLS leaf keys, self-test proves detection |

## 3. QG-03A checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Crate count | PASS | Exactly `sendust-core` and `sendust-napi`; checker rejects a third crate |
| Dependency direction | PASS | `sendust-napi` depends on core; core has no napi/Node dependency |
| Domain isolation | PASS | Checker rejects `lettre`/`napi`/transports references in domain modules |
| SMTP isolation | PASS | `lettre` confined to `transports/smtp`; nothing else may reference it |
| Public surfaces | PASS | Core facade exposes one diagnostic function; TS package exposes nothing yet |
| No speculative framework | PASS | No plugin registry, extension points, or extra transport |
| Test ownership | NOT APPLICABLE | Module-invariant tests arrive with the modules in P04/P06/P07; recorded as downstream, not claimed |

## 4. Not claimed

- No product behavior: P04 to P18 remain `NOT STARTED`.
- Runtime addon smoke tests for Node.js, Bun, and Deno are P05 deliverables; CI currently runs runtime-neutral TypeScript smoke only.
- Platform support (runtime × OS × CPU × ABI) is not claimed; P14-003/P14-004 own it.
- Performance, security audit, and QA gates are later phases.

## 5. Decision

**APPROVED.** The project owner reviewed this record and its CI evidence and
authorized the merge of `feat/p03-repository-skeleton` into `main` on 2026-11-13.
The implementation was written by an agent, so this approval is the required
human review step, not a self-approval: the reviewer is the project owner and
the evidence cited above is the CI run on the branch head.

Approval covers the repository baseline only. It does not approve P04 work,
runtime/platform support, or any later gate; a failed compile, harness, fixture,
or checker result on `main` reopens this gate.
