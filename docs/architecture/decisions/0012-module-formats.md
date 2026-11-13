# ADR-0012 — Dual ESM/CommonJS with a single shared implementation

**Status:** PROPOSED  
**Date:** 2026-11-13  
**Deciders:** Architecture review (P02-005)  
**Validated by:** P14-002, P14-004

## Context

Node.js consumers use both `import` and `require`. A native addon adds a second dimension: the loader must locate the platform package in both module systems. Dual-package hazards (two class identities, two native instances) break `instanceof` and can double-load the addon.

## Decision

- Provide ESM and CommonJS entry points through the `exports` map.
- Both entries delegate to one shared implementation module so there is one class identity and one native instance per process.
- Generated types are shared.
- The native loader is invoked once and cached by the shared module.
- If a consumer mixes `import` and `require` in one process, both resolve to the same implementation.

## Alternatives

1. **ESM-only.** Simplest, but excludes CommonJS consumers.
2. **CJS-only.** Works with `import` via interop, but loses clean ESM named exports and future compatibility.
3. **Two independent builds.** Risks duplicate class identities and duplicate native loads; rejected.

## Consequences

- Packaging is slightly more complex.
- Tests must cover both formats and a mixed-format process.
- Error classes must satisfy `instanceof` from both entry points.
- The exact file layout is decided in P14-002 and validated in P14-004.

## Evidence required

- ESM consumer test.
- CommonJS consumer test.
- Mixed-format test asserting one native instance and stable `instanceof`.
- `exports` map review.
