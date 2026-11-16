# ADR-0019 — Installable `@sendust/ts` package with the addon bundled

**Status:** ACCEPTED (project owner decision, 2026-11-16; implementation evidence P14-001 onward)  
**Date:** 2026-11-16  
**Supersedes:** the packaging shape in [ADR-0011](0011-platform-distribution.md) (root package plus platform optional packages). ADR-0011's rules that stand unchanged are: no `postinstall` download, no compiler at consumer install time, and no silent fallback when a binary is missing or wrong.  
**Related:** [ADR-0012](0012-module-formats.md), [ADR-0015](0015-multi-runtime-support.md), [ADR-0018](0018-public-trust-anchor.md), [distribution strategy](../../engineering/distribution-strategy.md)

## Context

The repository root is an npm workspace, not an installable package. Packing it would publish files that must never reach a consumer: the local `.env` (which can hold a live relay password, and `.npmignore` does not exclude it), the staged `ts/sendust.node` build artifact, the `docs/` tree, and uncompiled TypeScript source. The root `package.json` also declares no `main`, `types` or `exports`, so a consumer cannot import it at all.

ADR-0011 and the distribution strategy planned the root as the published `sendust` package, with platform packages as optional dependencies. That plan was never built, and P14-001 (the clean-consumer smoke) needs an installable artifact now.

## Decision

1. The installable artifact is a separate workspace package at `ts/`, named `@sendust/ts`, and built and packed on its own. The root stays a private workspace that is never published.
2. The package is compiled from `ts/src` to JavaScript and type declarations. Its `exports` map offers ESM `import` and CommonJS `require` over one shared implementation module, as ADR-0012 requires.
3. The staged native addon is bundled into the package. Its `files` allowlist contains only the compiled output, the addon for the target it was built for, the README and the licence. Tests, fixtures, docs, source tree, scripts, `.env` and any other workspace file are excluded by allowlist, not by a blocklist.
4. The loader resolves the bundled addon from the package root. A missing or wrong-architecture addon fails with a stable, actionable error. There is no fallback to another build.
5. This decision does not change the core crate boundaries, the TLS policy, or the error contract. It changes only how the TypeScript package is packed and resolved.

## Consequences

- The earlier `sendust` root-package plan is withdrawn for the MVP. The published name is `@sendust/ts`.
- A bundled addon covers one platform per package build. Multi-platform distribution is still required by ADR-0015, and it is not settled by this ADR. Choosing between bundling one addon per platform package and the ADR-0011 optional-package model is deferred to P14-003 and must be recorded in its own ADR before any second platform is published.
- Packing is a deliberate, separate step, so a stray workspace file cannot enter the artifact by default.
- The package must be verified from the packed tarball in a clean consumer project, never from the workspace, so that workspace-only resolution cannot hide a missing file.

## Alternatives

1. **Make the root `sendust` package installable.** Rejected by the owner: the root holds the workspace, test fixtures and build tooling, so every publish would need a risky blocklist to keep them out.
2. **Keep ADR-0011 unchanged and publish nothing until P14-002.** Rejected: P14-001 is the early smoke that exists to find these packaging faults now.
3. **Publish the addon as a separate package from day one.** Deferred to P14-003, where the platform set is decided.

## Evidence required

- A `npm pack` of `ts/` lists only the allowlisted files and contains no `.env`, `docs/`, `tests/`, `target/` or source-tree path.
- The packed tarball installs into a clean consumer project and imports under ESM and CommonJS, with one native instance and stable `instanceof`.
- A missing-addon fixture produces the stable error, and the message names the expected addon, not a source path.
- P14-001 smoke: one accepted send to the loopback fake relay from the installed artifact, then explicit close.
