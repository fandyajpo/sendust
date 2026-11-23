# Sendust — Dependency, Feature, and License Policy

**Status:** P03-002 deliverable. Version pins are owned by the [toolchain baseline](toolchain-baseline.md); this document owns dependency admission, feature selection, lockfile rules, and license review.

## 1. Scope

This policy applies to every Rust crate, npm package, GitHub Action, and test fixture source in the repository. It does not restate product architecture; ADR-0001, ADR-0002, ADR-0003, ADR-0004, and ADR-0016 remain authoritative for dependency boundaries.

A direct dependency may be added only through the review in section 6. Transitive dependencies are accepted with their direct parent and are audited at P11-004 and P16-002.

## 2. Direct Rust pins

| Crate | Pin | Kind | Features | Default features | Owner crate | Purpose |
| --- | --- | --- | --- | --- | --- | --- |
| `napi` | `=3.16.2` | normal | `napi8`, `async` | on | `sendust-napi` | Node-API 8 binding |
| `napi-derive` | `=3.7.0` | normal | — | on | `sendust-napi` | Exported binding macros |
| `napi-build` | `=2.6.0` | build | — | on | `sendust-napi` | Binding build setup |
| `idna` | `=1.1.0` | normal | `std`, `compiled_data` | off | `sendust-core` | IDNA/UTS #46 domain conversion for envelope addresses |
| `base64` | `=0.23.1` | normal | default | on | `sendust-core` | MIME transfer encoding (ADR-0010) |
| `libc` | `=0.2.191` | normal (unix targets only) | — | off | `sendust-core` | `O_NOFOLLOW`/`O_NONBLOCK` for the attachment reader (P08-002); already present transitively via `tokio` |
| `tokio` | `=1.54.1` | normal, dev | `time`, `sync`, `net`, `io-util`; dev-only `rt`, `macros` | off | `sendust-core` | Deadline enforcement and native TCP I/O; core never builds a runtime |
| `rustls` | `=0.23.46` | normal, dev | `std`, `tls12`, `ring` | off | `sendust-core` | Typed TLS error classification in the SMTP adapter and the TLS test harness |
| `tokio-rustls` | `=0.26.6` | normal | `tls12` | off | `sendust-core` | Async TLS stream for the native SMTP client (ADR-0017) |
| `webpki-roots` | `=1.0.12` | normal | default | on | `sendust-core` | Bundled trust store for the native client (ADR-0003) |
| `rustls-pki-types` | `=1.16.0` | normal | default | on | `sendust-core` | `ServerName` and certificate types for the native client |
| `mail-parser` | `=0.11.9` | dev | default | on | `sendust-core` | Independent MIME parser for conformance tests (P07-005); never ships |
| `rsa` | `=0.9.10` | normal | `std`, `pem` | off | `sendust-core` | RSASSA-PKCS1-v1_5 signatures for DKIM (ADR-0024) |
| `sha2` | `=0.10.9` | normal | `std`, `oid` | off | `sendust-core` | SHA-256 for the DKIM body and header hashes (ADR-0024) |

`sendust-core` declares no build script and no binding, runtime, or Node.js dependency. `sendust-napi` depends on `sendust-core` and never the reverse. No third-party SMTP engine is declared anywhere, in any kind of dependency, after PHASE-06 (ADR-0017).

`base64` and `idna` are the vetted primitives ADR-0010 requires; both are declared directly and add no new transitive weight. `idna` is declared directly so the core cannot silently drift from the resolver's choice.

`libc` was already in the build graph through `tokio`, `mio`, and `socket2`; it is declared directly only so the attachment reader can pass `O_NOFOLLOW` and `O_NONBLOCK`, which the standard library does not expose. The dependency is target-gated to unix and adds no new package to the lockfile.

`rsa` and `sha2` were admitted for DKIM signing under ADR-0024, after the review in section 6: RSA signing is a security primitive ADR-0010 forbids hand-rolling, both are RustCrypto crates under `MIT OR Apache-2.0`, and neither pulls OpenSSL, `native-tls`, or a second TLS stack. `rsa` is pinned to `0.9.10` rather than `0.10.0`: the 0.10 line was published three weeks before this pin and reworked the encoding and signature feature layout, while 0.9.10 is the mature line with the same PKCS#1 v1.5 semantics; `sha2` is pinned to `0.10.9` to match the `digest` generation `rsa` 0.9 is built on. `sha2`'s `oid` feature is required because a PKCS#1 v1.5 signing key has to name the digest it signs with.

Test-only dependencies (MIME verification, fake SMTP/TLS tooling, benchmarks) are introduced by the phase that needs them, pinned exactly at that time, recorded in this table with kind `dev`, and never enabled for published artifacts.

`tokio` is a normal pin because the native client bounds every SMTP step with the caller's timer; `rustls` and `tokio-rustls` are normal pins because the native client builds the TLS configuration and classifies the typed rustls cause to distinguish `TLS_CERTIFICATE` from `TLS_HANDSHAKE`. They are the same versions the test harness uses, so the fixtures exercise the production TLS stack.

## 3. Feature and transitive policy

- NO third-party SMTP engine may be reintroduced; that requires a superseding ADR. Pooling and connection reuse are outside MVP scope.
- TLS uses the rustls `ring` provider with `webpki-roots`; certificate and hostname verification are always on. No configuration, feature, or method that disables verification may be added.
- Feature flags are declared once at workspace level. Enabling an optional capability per crate to work around a pin is a policy violation, not a convenience.
- A dependency that pulls OpenSSL, `native-tls`, or a second TLS stack blocks the change.

## 4. Lockfiles

- `Cargo.lock` is committed for the workspace and is generated by the pinned toolchain in `rust-toolchain.toml`. A lockfile resolved by another toolchain is not evidence.
- CI resolves with `--locked` once the lockfile is committed; until then the lockfile is a named pending item, never silently regenerated in a release job.
- One root `package-lock.json` is authoritative for npm. `pnpm-lock.yaml`, `yarn.lock`, and `bun.lockb` must not be committed.
- No dependency is installed by a postinstall script, and no native binary is downloaded at install time.

## 5. Prohibited capabilities

The following are out of scope for MVP regardless of the crate that provides them:

- a third-party SMTP engine or protocol client (Sendust owns the client under ADR-0017; TLS, I/O and encoding primitives remain third-party);
- insecure TLS toggles, plaintext fallback, or certificate/hostname bypass;
- URL, HTTP, or remote attachment fetching;
- connection pooling, automatic retry, internal queues, workers, schedulers, or persistence;
- a third Rust crate, workspace, plugin registry, or alternate transport;
- Node.js, `napi`, or JavaScript runtime types inside `sendust-core`;
- SMTP, MIME, validation, or delivery policy inside `sendust-napi` or `ts/src`;
- `any` in public TypeScript contracts;
- telemetry or observability SDKs that could carry message content or credentials.

## 6. Adding a dependency

1. State the requirement or constraint ID that needs it and why the standard library and existing pins cannot satisfy it.
2. Check license compatibility against section 7 and record the license, version, and source.
3. Check maintenance status, release cadence, advisory history, and transitive weight.
4. Confirm it does not cross the core/binding boundary, add runtime-specific behavior to shared code, or duplicate SMTP/MIME/validation policy.
5. Pin the exact version and the smallest feature set; verify `cargo tree -e features` and `cargo tree -d`.
6. Update this document, the [toolchain baseline](toolchain-baseline.md) when a version pin changes, the owning phase plan, and `Cargo.lock` in the same change.
7. Add the license to the inventory in section 8 and re-run the architecture and documentation checks.

A dependency without a recorded justification is rejected in review even if it compiles.

## 7. Allowed licenses

Apache-2.0, MIT, BSD-3-Clause, ISC, Unicode-3.0, and CDLA-Permissive-2.0 are pre-approved for direct dependencies. Any other license requires an explicit decision recorded in this section before the dependency is merged, and must be compatible with distributing the native addon and the npm package.

## 8. License inventory template

Every direct dependency must have one row. The inventory is completed before release at P16-002.

| Crate | Version | License | Source | Introduced by |
| --- | --- | --- | --- | --- |
| `napi` | `3.16.2` | MIT | https://crates.io/crates/napi | P03-002 |
| `napi-derive` | `3.7.0` | MIT | https://crates.io/crates/napi-derive | P03-002 |
| `napi-build` | `2.6.0` | MIT | https://crates.io/crates/napi-build | P03-002 |
| `idna` | `1.1.0` | MIT OR Apache-2.0 | https://crates.io/crates/idna | P04-002 |
| `base64` | `0.23.1` | MIT OR Apache-2.0 | https://crates.io/crates/base64 | P04-003 |
| `tokio` | `1.54.1` | MIT | https://crates.io/crates/tokio | P04-005 |
| `rustls` | `0.23.46` | Apache-2.0 OR ISC OR MIT | https://crates.io/crates/rustls | P04-005 |
| `libc` | `0.2.191` | MIT | https://crates.io/crates/libc | P08-002 (target-gated; already transitive via `tokio`) |
| `mail-parser` | `0.11.9` | Apache-2.0 OR MIT | https://crates.io/crates/mail-parser | P07-005 (dev-only) |
| `rsa` | `0.9.10` | MIT OR Apache-2.0 | https://crates.io/crates/rsa | 1.2.0 (ADR-0024) |
| `sha2` | `0.10.9` | MIT OR Apache-2.0 | https://crates.io/crates/sha2 | 1.2.0 (ADR-0024) |

## 9. npm development tool pins

Development tools never ship to consumers and are installed from one root lockfile with exact versions and no caret or tilde ranges.

| Tool | Pin | Scope |
| --- | --- | --- |
| npm | `12.2.2` | package manager and lockfile owner |
| TypeScript | `7.0.2` | strict type checking |
| Biome | `2.6.7` | linting and formatting |
| Vitest | `5.1.0` | TypeScript unit and contract tests |
| Nodemailer | `10.6.0` | benchmark comparison only; never shipped (`bench/`, root devDependency) |

These versions are enforced by `scripts/check-docs.mjs`, which compares this baseline, `ts/package.json`, `rust-toolchain.toml`, and `.github/workflows/ci.yml`.

## 10. Evidence commands

```text
cargo metadata --format-version 1 --no-deps
cargo tree -e features
cargo tree -d
node scripts/check-architecture.mjs
node scripts/check-docs.mjs
npm ci
npm ls --all
```

These commands are the dependency evidence for P03-003 and later gates. They are **NOT RUN** until an execution environment runs them; passing compilation alone is not license, advisory, or feature-policy evidence.
