# ADR-0020 — Per-platform native packages

**Status:** ACCEPTED (P14-003 decision; supersedes ADR-0019's single-bundle layout)  
**Date:** 2026-11-16  
**Related:** [ADR-0019](0019-installable-typescript-package.md), [platform matrix](../../qa/platform-matrix.md), ADR-0011

## Context

ADR-0019 made `ts/` the publishable package and bundles one `sendust.node` with it. One tarball cannot carry a binary for every platform, and shipping all binaries in one package would multiply the download for every consumer.

## Decision

Publish **per-platform packages**, resolved through optional dependencies:

| Platform key | Package |
| --- | --- |
| `linux-x64-gnu` | `@sendust/ts-linux-x64-gnu` |
| `linux-arm64-gnu` | `@sendust/ts-linux-arm64-gnu` |
| `linux-x64-musl` | `@sendust/ts-linux-x64-musl` |
| `linux-arm64-musl` | `@sendust/ts-linux-arm64-musl` |
| `macos-x64` | `@sendust/ts-macos-x64` |
| `macos-arm64` | `@sendust/ts-macos-arm64` |
| `windows-x64` | `@sendust/ts-windows-x64` |

The two musl keys were added on 2026-11-22: Alpine is the default small Node
image, and every install there failed closed. They follow the same rule as the
rest — the binary is built and loaded inside a musl container on the runner of
that architecture, never cross-compiled.

- Each platform package contains only `sendust.node` and a `package.json` declaring `os`, `cpu`, `files`, `main` (unused but required by npm tooling) and no scripts.
- `@sendust/ts` lists them under `optionalDependencies`, so an install on an unsupported platform succeeds but has no addon.
- The loader resolves, in order: the bundled `sendust.node` (source tree and local build), then the platform package named by `process.platform` / `process.arch` (plus glibc vs musl on Linux, read from `process.report.getReport().header.glibcVersionRuntime`), then a stable `CONFIG_NATIVE_UNAVAILABLE` naming the expected package.
- No postinstall download, no compiler at install time, and no silent fallback to a different build.

## Consequences

- Downloads stay small; a consumer installs exactly one binary.
- The loader must know the platform-to-package mapping and must fail clearly when the platform is unsupported or the optional install was skipped.
- `npm` skips optional dependencies that fail to install (for example a wrong-arch package), which the loader treats as "missing addon", never as "unsupported build".
- The platform packages are published only when P14-004 has validated them for their runtime pins; absence from the registry is not a support claim.

## Alternatives

1. **One package with all binaries** — simpler resolution, but every consumer downloads every platform (several MiB each).
2. **Single bundled binary (ADR-0019)** — works for one platform only; cannot be published to npm for a multi-platform claim.
