# Maintainer guide

**Status:** maintainer documentation for the MVP. It describes the repository as
it is validated today; nothing here is a feature or platform claim beyond
[the support matrix](../qa/platform-matrix.md).

## 1. Layout

```text
crates/sendust-core/     Rust core: validation, addressing, message model,
                         MIME, caps, lifecycle, delivery, native SMTP client
crates/sendust-napi/     Node-API adapter: owned copies, async Tokio, errors
ts/src/                  Runtime-neutral facade: transport, errors, results,
                         limits, loader
ts/test/                 TypeScript contract and transport tests (Vitest)
tests/                   Rust integration suites, runtime smokes, docs samples,
                         TLS fixtures
examples/                Runnable examples (Rust and Node.js)
scripts/                 Build, verification, and static checkers
docs/                    Product, architecture, ADRs, engineering, QA, guides
```

Rules that must hold:

- **Exactly two Rust crates.** The core has no Node linkage; the adapter depends
  on the core, never the reverse. `scripts/check-architecture.mjs` enforces it.
- **The TypeScript facade stays runtime-neutral.** No static `node:` imports
  outside `ts/src/loader.ts`; the loader is the only runtime-specific file.
- **Generated output is never edited.** `ts/dist`, `artifacts/`, and `target/`
  are build products.

## 2. Commands

```text
npm ci                                  # install the workspace and its lockfile
npm run build:native                    # build the addon into ts/sendust.node
npm run build --workspace ts            # compile dist/ (ESM and CommonJS)
npm --workspace ./ts run test           # 61 TypeScript tests
npm run verify                          # addon, package, tests, smokes, artifact,
                                        # publish manifest, clean-consumer checks
npm run verify:platform                 # packed tarball under Node/Bun/Deno
npm run prepare:publish                 # artifacts/publish/sendust-ts-*.tgz
cargo test --workspace --locked         # Rust unit and integration suites
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
node scripts/check-architecture.mjs
node scripts/check-docs.mjs
node scripts/check-doc-claims.mjs       # guides vs code and forbidden claims
node scripts/check-secrets.mjs
```

Runtime smokes and doc samples:

```text
node tests/runtime/smoke.mjs            # the addon directly
node tests/runtime/facade.mjs           # through the loader
node tests/runtime/behaviour.mjs        # accepted/rejected/unknown/BUSY/close/limits
node tests/docs/quickstart.mjs          # the quick start guide's sample
```

Bun and Deno run the same files with their own runtime; the exact pins are in
[the toolchain baseline](../engineering/toolchain-baseline.md).

## 3. Test harness and fixtures

- **Loopback only.** Every SMTP and TLS test uses a local fake server; no test
  contacts a real relay or needs credentials.
- **TLS fixtures** live in `tests/fixtures/tls/`: `ca.crt` plus valid, wrong-host,
  expired, self-signed, incomplete-chain, and malformed material. The facade
  tests trust `ca.crt` through `tlsRootCertificate` and prove that a missing
  anchor fails closed.
- **Rust harness** (`crates/sendust-core/tests/support`) scripts replies and
  faults; its self-tests are part of the Rust suite.
- **No sleeps for sequencing.** Tests wait on events (a connection gate) rather
  than sleeping; the descriptor probes allow a bounded settle instead of a fixed
  delay.
- **Docs are executed.** `tests/docs/quickstart.mjs` runs the guide's example, so
  a doc change that no longer matches the package fails CI.

## 4. What CI proves

| Job | Proves |
| --- | --- |
| Rust workspace | Formatting, clippy, unit and integration suites, harness self-tests |
| TypeScript (Node 22/24/26) | Lint, format, typecheck, the Vitest suite |
| Native binding (Node 22) | Built addon, binding tests, GC probe, runtime smokes, doc sample |
| Native artifact (5 targets) | Native build and load, 61-test suite, tarball install with platform selection, Bun/Deno checks, behaviour harness, publish manifest |
| Architecture and documentation | Module boundaries, links, registry parity, doc claims, secrets |
| Bun/Deno entry points | The shared package imports and runs without native claims |

## 5. Documentation rules

- `docs/guides/` is user-facing. `docs/architecture/`, `docs/engineering/`,
  `docs/planning/`, and `docs/qa/` are the design and evidence record.
- Every guide states what is **not** provided when the topic invites a wrong
  assumption (delivery guarantees, retries, pooling, SMTPUTF8, URL attachments).
- `node scripts/check-doc-claims.mjs` fails when a guide names an unknown error
  code, drifts from the cap or timeout numbers in the code, promises delivery or
  automatic retries, or shows a send without an explicit close.
- Keep the [docs index](../README.md) row list in step with new files.

## 6. Release inputs

Publication is P16/P17, not this phase. The inputs are:

- `npm run prepare:publish` — the facade tarball whose manifest adds the five
  platform `optionalDependencies` pinned to the facade version.
- `artifacts/sendust-<platform>-<arch>[-<libc>].node` — one per validated target.
- [Platform validation](../qa/platform-validation.md) and
  [the toolchain baseline](../engineering/toolchain-baseline.md) — the support
  statement and exact pins.
- `CHANGELOG`, version bump, and tag policy — owned by P16.

Adding a platform means: a native runner that can load the build, a row in the
matrix, a `PLATFORM_PACKAGES` entry in `ts/src/loader.ts`, coverage in the
selection check, and evidence in the validation report. Until then the platform
is unsupported and absent from publication inputs.
