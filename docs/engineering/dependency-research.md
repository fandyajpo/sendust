# Technical Research — 2026-11-13

**Phase:** 01  
**Status:** P01 source/API research complete; see the pinned [lettre validation report](../research/lettre-0.11.23-validation.md) and [Node runtime validation plan](../research/node-runtime-validation-plan.md). Cargo, fake-relay, and native runtime probes remain downstream implementation gates.  
**Rule:** This document records what was observed, from where, and what remains unverified. It is not a substitute for P01-001 through P01-005.

## 1. Method

- Repository inspected first: documentation and planning repository; no application source or existing implementation toolchain was present in the audited snapshot.
- Sources consulted on 2026-11-13:
  - napi-rs v3 official documentation and source for async functions, compatibility, Buffer, and AsyncTask.
  - lettre v0.11.23 pinned source: Cargo.toml, AsyncSmtpConnection, commands, response, error, authentication, and TLS builder.
- No dependencies were installed, no network probes were run against SMTP servers, and no application code was generated. The audit container has Node.js 22.16.0 but no rustc/cargo.
- Version claims below are **observed on those pages**, not independently verified against lockfiles. P01-001 through P01-004 must verify pinned source and features.

## 2. Observed platform facts

| Fact | Observation | Source | Status |
| --- | --- | --- | --- |
| napi-rs major version | v3 documentation; `napi new` scaffolds v3 | napi.rs getting started | Observed |
| napi-rs CLI Node requirement | `>=23.5.0 || ^22.13.0 || ^20.17.0`; Node 22.13+ or 24+ recommended | napi.rs support and compatibility | Observed |
| napi-rs Rust requirement | Rust 1.88 or newer | napi.rs getting started | Observed |
| Node-API levels offered by scaffold | 1 through 9; default 4 | napi.rs support and compatibility | Observed |
| Async support | `async fn` runs on napi-managed Tokio runtime; `async` or `tokio_rt` feature required | napi.rs async fn | Observed |
| Panic behavior | Uncaught panic in sync callback can terminate process; `catch_unwind` available only with unwind strategy | napi.rs errors and panics | Observed |
| Distribution model | Root package plus platform optional packages; loader selects by OS/CPU/libc | napi.rs release deep dive | Observed |
| Release commands | `create-npm-dirs`, `build`, `artifacts`, `pre-publish`, `npm publish` | napi.rs release deep dive | Observed |
| Release atomicity | Multi-platform publication is not atomic; partial failure requires recovery procedure | napi.rs release deep dive | Observed |
| lettre version | 0.11.23, published 2026-10-10 | docs.rs crate page | Observed |
| lettre license | MIT | docs.rs crate page | Observed |
| lettre async runtime | `tokio1` feature with `Tokio1Executor`; wrong runtime causes panic | docs.rs lettre | Observed |
| lettre TLS backends | rustls 0.23 with `ring`/`aws-lc-rs`/provider selection; verification backend must be chosen (`rustls-platform-verifier`, `rustls-native-certs`, or `webpki-roots`) | docs.rs lettre | Observed |
| lettre MIME/message builder | Typed `Message` builder, header types, multipart support | docs.rs lettre | Observed |
| lettre error surface | `lettre::error`, transport SMTP error module | docs.rs lettre | Observed |
| mail-parser version | 1.0.1, published 2026-11-02 | docs.rs mail-parser | Observed |
| mail-parser license | Apache-2.0 OR MIT | docs.rs mail-parser | Observed |

## 3. Dependency evaluation

### 3.1 `lettre` 0.11.23 — SMTP and message primitives

- **Purpose:** SMTP transport, TLS/STARTTLS, authentication, address types, message builder, MIME encoding.
- **Why Sendust needs it:** Writing and maintaining an SMTP client, TLS integration, AUTH mechanisms, MIME encoding, and reply parsing is a large correctness and security surface. lettre is the most complete Rust option with async Tokio support.
- **Alternatives:**
  - `mail-send` — simpler SMTP, less actively shaped around async ecosystems; would require more MIME work.
  - `samotop` — server-oriented; not a client library.
  - Custom SMTP client — maximum control over stage/recipient observability, but high correctness, TLS, and maintenance risk; rejected unless P01-003 proves a required capability is inaccessible.
- **Maintenance:** Active release observed (0.11.23 in October 2026). Maintenance cadence must be confirmed in P01-001.
- **Compatibility:** Requires a matching async executor feature. Tokio 1 is available. TLS feature combinations are non-trivial and must be validated (P01-002).
- **Security:** TLS verification must be explicitly configured. Defaults and feature flags must be audited. Pin exact version and review advisories at P11-004.
- **MVP:** Required.

### 3.2 `napi-rs` v3 — Node-API binding

- **Purpose:** Generate Node-API bindings, TypeScript declarations, async Promise exports, platform packages.
- **Why Sendust needs it:** It is the current, maintained Rust-to-Node binding toolchain with async support, generated types, and a documented multi-platform release model.
- **Alternatives:**
  - `neon` — mature, but different async and packaging model; more manual type generation.
  - `node-bindgen` — less active and less complete packaging story.
  - Raw Node-API via `node-sys` — maximum control, high unsafe burden.
- **Maintenance:** v3 is the current documented line; release workflow is documented.
- **Compatibility:** CLI needs a modern Node version at build time; runtime compatibility is governed by Node-API level. The scaffold defaults to Node-API 4; Sendust selects Node-API 8 as the design floor. Exact feature-floor compilation and runtime tests remain P05/P14 gates.
- **Security:** Native code; panic safety and memory ownership are critical. `catch_unwind` limits must be understood.
- **MVP:** Required.

### 3.3 `rustls` 0.23 with `ring` and `webpki-roots`

- **Purpose:** TLS with a pure-Rust stack and bundled trust anchors.
- **Why Sendust needs it:** Avoids OpenSSL platform variance and gives deterministic trust behavior across prebuilt binaries.
- **Alternatives:**
  - `native-tls` — uses platform TLS; varies by OS and adds platform dependencies.
  - `rustls-platform-verifier` — uses OS trust store; better enterprise CA integration, more platform behavior to validate.
  - `rustls-native-certs` — loads OS roots; intermediate option.
- **Maintenance:** rustls is actively maintained; provider/feature churn is a known risk.
- **Compatibility:** `ring` may have platform build constraints; `webpki-roots` is deterministic but ignores enterprise-installed CAs. This is a product trade-off to document.
- **Security:** Verification must be mandatory. Root store choice affects enterprise users.
- **MVP:** Required, with P01-002 validating exact features.

### 3.4 `tokio` 1

- **Purpose:** Async runtime for lettre futures.
- **Why Sendust needs it:** napi-rs provides a Tokio runtime for `async fn`; lettre supports `Tokio1Executor`.
- **Alternatives:** `async-std` (lettre supports it, but napi-rs integration is Tokio-oriented).
- **Compatibility:** Must not create a second runtime. Blocking file reads must use a blocking pool.
- **MVP:** Required transitively through napi/lettre; not directly exposed.

### 3.5 `mail-parser` 1.0.1 — test-only MIME verification

- **Purpose:** Parse emitted MIME in tests to verify structure, headers, encoding, and attachments.
- **Why Sendust needs it:** Hand-rolled MIME assertions are brittle. An independent parser catches composition bugs.
- **Alternatives:** Python/Node MIME parsers in test harness; raw byte assertions.
- **Maintenance:** Active release observed (1.0.1, November 2026).
- **Security:** Test-only; not shipped in the addon.
- **MVP:** Test dependency.

### 3.6 `serde` / `serde_json`

- **Purpose:** Structured error details and possibly option conversion.
- **Why Sendust might need it:** Machine-readable details across the FFI boundary.
- **Alternatives:** Manual `napi` object construction.
- **Trade-off:** Extra compile time and binary size. Prefer manual conversion unless details become complex.
- **MVP:** Optional; decide at P05-001.

### 3.7 `thiserror`

- **Purpose:** Ergonomic core error definitions.
- **Alternatives:** Manual `Display`/`Error` implementations.
- **Trade-off:** Small dependency; does not cross FFI. Acceptable if it reduces boilerplate without hiding mapping.
- **MVP:** Optional.

### 3.8 `tracing`

- **Purpose:** Internal diagnostics.
- **Alternatives:** `log`.
- **Security:** Must never record credentials, bodies, or attachment bytes. Default off or no-op in the addon.
- **MVP:** Optional; observability is post-MVP.

### 3.9 `criterion`

- **Purpose:** Benchmarks for MIME composition and send-path overhead.
- **MVP:** Phase 13 only, dev dependency.

## 4. Node.js and Node-API compatibility

- Node-API provides ABI stability: a binary built against level N loads on later runtimes providing level N.
- That guarantee does not cover OS/CPU/libc/deployment-target compatibility or alternate runtime bugs.
- napi-rs documentation explicitly warns against “all Node versions” claims and recommends naming tested versions.
- Candidate runtimes for Sendust: Node.js 22, 24, 26. They are **candidates**, not claims, until P01-004 and P14-004 produce evidence.
- Node 26 may be pre-release at implementation time; it must not be advertised as supported without a blocking CI run.

## 5. Package and release research

- napi-rs publishes a root package plus per-platform optional packages.
- The loader resolves by `os`, `cpu`, and `libc`.
- Publication is not atomic. A failure can leave platform packages published without a root package, or vice versa.
- The release workflow must gate on artifact completeness because the CLI warns and continues when an expected artifact is missing.
- Provenance and post-publication verification are documented requirements.
- Recovery uses the unchanged artifacts; binaries must never be replaced under the same version.

## 6. Research conclusions that constrain the plan

1. Use lettre 0.11.23's public lower-level AsyncSmtpConnection API; do not write a custom SMTP client.
2. Use napi-rs v3 with the napi-managed Tokio 1 runtime; do not create a second runtime.
3. Node-API 8 is the approved ABI design floor; compile/runtime validation remains P05/P14 work.
4. Use the source-reviewed lettre feature set: defaults off, smtp-transport, tokio1-rustls, ring, webpki-roots. Cargo build/tree validation remains P03-002.
5. Treat Node 22/24/26 as candidates only; see the [runtime validation plan](../research/node-runtime-validation-plan.md).
6. Per-recipient RCPT responses are observable via command(Rcpt::new(...)) and Error::status(); timeout reporting is limited to coarse public-call boundaries.
7. lettre's inner connect timeout bounds TCP attempts only; an outer connect deadline must include DNS, greeting, EHLO, and TLS negotiation.
8. Plan for non-atomic publication and explicit artifact gates. Use mail-parser only in tests.

## 7. What this research does not establish

- That the selected Cargo feature graph compiles on every target.
- That the napi-managed runtime satisfies lettre on every candidate Node version.
- That all proposed platform targets can be built and runtime-tested in CI.
- That fake-relay behavior matches the inspected public API for every fault path.
- That TLS, MIME, filesystem safety, and timeout tests pass in the eventual implementation.

These are explicit downstream implementation gates. Source inspection establishes the architecture choice, not executable correctness.
