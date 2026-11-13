# Phase 14 — Distribution

**Phase status:** NOT STARTED  
**Phase dependency:** Phase 05, Phase 03 (early smoke); final matrix consumes Phase 12 and Phase 13 through task dependencies  
**Companion references:** [release strategy](../engineering/release-strategy.md), [testing strategy](../engineering/testing-strategy.md), [security strategy](../engineering/security-strategy.md), [performance strategy](../engineering/performance-strategy.md), [system design](../architecture/system-design.md)

## Objective

Make the package installable and loadable from a clean consumer project on supported platforms and required Node.js, Bun, and Deno runtimes. Distribution must preserve the native binding, two-crate build model, and security posture without introducing install-time network surprises or undocumented platform claims.

## Scope

In scope: package metadata, native artifact layout, clean-install smoke, platform/architecture matrix, load-time validation, and distribution gate. Out of scope: actual publication (Phase 17), release notes (Phase 16), performance optimization, and adding features. Planning only; no package installation is performed now.

## Tasks

### P14-001 — Early distribution smoke from a clean consumer

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P05-005, P03-005
- **Description:** Define an early smoke that builds the napi binding and TypeScript package from the workspace, installs the resulting local artifact into a clean temporary consumer, imports it, sends one message to a loopback fake relay, and closes explicitly. This is a structural smoke, not the final compatibility matrix. Verify no source-tree path leakage and no reliance on workspace-only resolution.
- **Evidence criteria:** Smoke record shows clean import, one accepted fake-relay send, explicit close, and no external network. Package resolves from the installed artifact, not the workspace. Failure diagnostics identify missing native binary versus TypeScript resolution issues.

### P14-002 — Define package metadata, exports, and native artifact layout

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P14-001
- **Description:** Specify package name, version source, `main`/`types`/`exports`, engine ranges, optional platform packages, native binary naming, and loader resolution order. Document exact tested Node.js, Bun, and Deno versions; none is a support claim until blocking tests pass. Define behavior when the platform binary is absent: a clear load error, never a silent fallback to an unsupported build.
- **Evidence criteria:** Metadata contract lists every field and resolution order. Engine range wording is honest. Missing-binary fixture produces a stable error. Exports do not expose internal core or napi paths. Package contents exclude tests, source maps, and secrets.

### P14-003 — Specify cross-platform build and prebuilt artifact matrix

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P14-002
- **Description:** Define target platforms and architectures, build toolchain requirements, and prebuilt artifact naming. Cover macOS, Linux, and Windows where supported, including arm64/x64 variants. Specify how rustls `ring`/`webpki-roots` and Node-API 8 are built consistently. Validate the runtime × OS × CPU × ABI matrix, including Deno local node_modules and --allow-ffi. Record unsupported targets explicitly and prevent accidental “best effort” publication.
- **Evidence criteria:** Matrix lists OS, arch, libc where relevant, Node-API level, artifact name, and support status. Each target has a build recipe and validation owner. Unsupported targets are absent from publication inputs. No target weakens TLS verification or changes core behavior.

### P14-004 — Validate clean installs and runtime loading across the matrix

- **Size:** L
- **Status:** NOT STARTED
- **Prerequisites:** P14-003
- **Description:** Define clean-environment tests for each supported target and candidate Node version: install from local artifacts, import, verify native binary selection, run a fake-relay accepted send, exercise BUSY, run close, and confirm no post-close work. Include missing-binary and wrong-architecture negative tests.
- **Evidence criteria:** Matrix records pass/fail/unsupported per OS, arch, and Node candidate. Negative tests produce actionable errors. Loaded binary path and version are recorded. No test contacts a real relay or requires a compiler at consumer install time unless explicitly documented.

### P14-005 — Final distribution gate using QA and performance evidence

- **Size:** M
- **Status:** NOT STARTED
- **Prerequisites:** P14-004, P12-005, P13-005
- **Description:** Run the final distribution matrix after QA and performance gates. Confirm clean installs, native loading, accepted/unknown/BUSY/close behavior, bounded resource use, and honest support declarations. Reconcile the early smoke with the final matrix and document any target dropped.
- **Evidence criteria:** Gate package includes per-target install/load/send evidence, candidate Node results, negative tests, performance limitations, and support matrix. No unsupported target is advertised. No install-time secret or external network dependency exists. Phase 16 may consume P14-005.

## Technical implementation details

Distribution must ship the two-crate implementation through the napi binding: `TS wrapper -> napi -> core -> lettre`. Core remains free of Node linkage. Native artifacts target Node-API 8 and are loaded by `ts/src`. Candidate Node 22/24/26 must be validated, not assumed. `lettre 0.11.23` and rustls `ring`/`webpki-roots` must be built with verification enabled. The package must not add pooling, retries, SMTPUTF8, URL attachments, or required AbortSignal. One active send returns `BUSY`; close is explicit and bounded by 125s. Timeouts remain connect 10s, operation 30s, total 120s. Caps remain 100 recipients, 10 MiB per attachment, 20 MiB aggregate raw, 30 MiB wire. Consumer installs must not fetch URLs or require credentials; attachment filesystem access remains disabled by default.

## Dependencies

P14-001 depends on P05-005 and P03-005 for early smoke. P14-002 depends on P14-001; P14-003 on P14-002; P14-004 on P14-003. P14-005 depends on P14-004 plus P12-005 and P13-005. Phase 14 itself has Phase 05/03 prerequisites; the final matrix consumes Phase 12/13 evidence via task dependencies, avoiding phase cycles. Phase 16 depends on Phase 14, Phase 12, Phase 15, and Phase 13.

## Risks and edge cases

Native binary resolution can silently pick the wrong architecture. Engine ranges may overpromise Node support. Optional platform packages can create install-time network failure modes. Prebuilt binaries can drift from tested source. macOS signing/quarantine and Linux libc differences can break loading. A consumer without a compiler must get a clear error, not a broken import.

## Testing requirements

Run early clean-consumer smoke with a local artifact and fake SMTP: accepted send, BUSY, close, and missing-binary negative. Final matrix covers every supported OS/arch and every pinned Node.js, Bun, and Deno version: install, import, binary selection, accepted send, mixed RCPT ADR behavior, explicit final DATA `451` rejection and missing-final-reply-after-DATA unknown, BUSY, double close, and wrong-architecture/missing-binary errors. No external network, no real credentials, no publication.

## Acceptance criteria

- A clean consumer can install and import the package from local artifacts on every supported target.
- Native binary selection is correct; missing/wrong artifacts fail clearly.
- Node.js, Bun, and Deno support is validated and honestly declared per exact platform combination.
- No install-time network, secret, or feature-contract violation exists.
- Early smoke and final matrix are reconciled.

## Deliverables

Early smoke report, package metadata contract, cross-platform artifact matrix, clean-install/load test matrix, support declaration, and final distribution gate package.

## Completion gate

`PHASE-14` is approved only when P14-005 proves clean install/load behavior across the declared matrix, honest support claims, and no distribution-induced contract or security regression. Phase 16 may begin only after this gate.

## Multi-runtime amendment requirements

P14 must test the intersection of each claimed runtime and platform target. Required runtimes are Node.js, Bun, and Deno. A target must pass clean install, native load, correct binary selection, accepted fake-SMTP send, mixed-recipient/final-DATA outcomes, Buffer ownership, BUSY, close/shutdown, TLS verification, and error mapping. Deno clean installs must verify local node_modules and --allow-ffi. No target may be published based only on compilation or upstream framework claims.

## Status

NOT STARTED.
