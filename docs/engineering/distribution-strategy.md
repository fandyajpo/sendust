# Native Distribution Strategy

**Applies to:** P14, P16, P17.  
**Related:** [ADR-0011](../architecture/decisions/0011-platform-distribution.md), [release strategy](release-strategy.md), [requirements](../product/requirements.md) C-05, C-06, C-26, C-27.

## 1. Distribution model

Sendust ships prebuilt native addons as npm packages using the napi-rs model:

- One root package: `sendust`.
- One optional package per platform, for example:
  - `@sendust/darwin-arm64`
  - `@sendust/darwin-x64`
  - `@sendust/linux-x64-gnu`
  - `@sendust/linux-arm64-gnu`
  - `@sendust/linux-x64-musl`
  - `@sendust/linux-arm64-musl`
  - `@sendust/win32-x64-msvc`
- The root package lists exact-version optional dependencies.
- npm selects the matching package by `os`, `cpu`, and `libc`.
- The loader resolves the native binary at runtime.

Consumers never need a Rust toolchain, and no `postinstall` download is used.

The exact package scope and names are finalized in P14-002. If `sendust` is unavailable or a scope is required for platform packages, the scope decision is recorded before the first publish.

## 2. Proposed support matrix

| OS | Arch | Libc | Target triple | Status |
| --- | --- | --- | --- | --- |
| macOS | arm64 | — | `aarch64-apple-darwin` | Proposed |
| macOS | x64 | — | `x86_64-apple-darwin` | Proposed |
| Linux | x64 | glibc | `x86_64-unknown-linux-gnu` | Proposed |
| Linux | arm64 | glibc | `aarch64-unknown-linux-gnu` | Proposed |
| Linux | x64 | musl | `x86_64-unknown-linux-musl` | Proposed |
| Linux | arm64 | musl | `aarch64-unknown-linux-musl` | Proposed |
| Windows | x64 | MSVC | `x86_64-pc-windows-msvc` | Proposed |

A target is **claimed** only when P14-004 builds it, installs it in a clean consumer, loads it, and sends through a fake relay on a real or faithfully emulated environment. Otherwise it is labeled unsupported and excluded from publication.

## 3. Node.js support

- Node-API level: 8 proposed.
- Runtime candidates: Node.js 22, 24, 26.
- Only versions with a blocking CI runtime test are claimed.
- Node 26 is optional for MVP because of pre-release risk.
- Alternate runtimes (Bun, Deno) are not claimed; they are untested unless P14 records otherwise.

## 4. Build requirements

| Target | Build host | Toolchain notes |
| --- | --- | --- |
| macOS arm64/x64 | macOS runners | Xcode command line tools; cross-arch build on macOS |
| Linux glibc x64/arm64 | Linux runners or cross | glibc floor recorded; linker/SDK validated |
| Linux musl x64/arm64 | Linux with musl toolchain or cross | Static runtime considerations |
| Windows x64 MSVC | Windows runner | MSVC build tools; runtime dependency documented |

Each target records:

- Rust toolchain version.
- napi-rs CLI version.
- Node-API level.
- Linker/SDK version.
- Minimum OS or glibc floor.
- Artifact name and checksum.

## 5. Artifact layout

```text
sendust/                      # root package
├── package.json              # optionalDependencies, exports, engines
├── index.js                  # loader
├── index.d.ts                # public types
└── ...

@sendust/darwin-arm64/
├── package.json              # os: ["darwin"], cpu: ["arm64"]
└── sendust.darwin-arm64.node
```

The exact binary name and loader path are fixed in P14-002. The loader must:

- Detect the current platform and libc.
- Load the matching package.
- Produce a clear error if the package or binary is missing.
- Never silently load a wrong-architecture binary.
- Report the loaded path and version in diagnostics.

## 6. Package metadata

Required fields:

- `name`, `version`, `description`, `license`, `repository`.
- `main`, `types`, and `exports` with `import`, `require`, and `types`.
- `engines.node` matching the validated runtime range.
- `optionalDependencies` with exact versions.
- `os`, `cpu`, `libc` constraints in platform packages.
- `files` allowlist excluding tests, sources, maps, and secrets.

Unknown option keys, unsupported platforms, and missing binaries must fail with actionable errors.

## 7. Clean-install validation

For every claimed target:

1. Build the artifact in CI.
2. Publish to a local registry or install the tarball into a clean temp project.
3. Import via ESM and CommonJS.
4. Assert the loaded binary path and version.
5. Send one message through a fake relay and assert `accepted`.
6. Exercise `BUSY`, close, and post-close behavior.
7. Run missing-binary and wrong-architecture negative tests.
8. Record results in the distribution matrix.

## 8. Publication gates

Before publication:

- Every configured target directory exists.
- Every directory contains exactly the expected `.node` file.
- No unexpected binary name or target suffix exists.
- Platform packages and root share the exact version.
- Root `optionalDependencies` reference the exact versions.
- Provenance and SBOM are attached.
- No test, source map, or secret is included.

Publication is not atomic. If a platform package fails after others are published, follow the recovery procedure in [release-strategy.md](release-strategy.md): publish missing packages from unchanged artifacts, never rebuild or replace bits under the same version.

## 9. Platform-specific risks

| Risk | Mitigation |
| --- | --- |
| macOS quarantine/signing | Document Gatekeeper behavior; test downloaded artifacts |
| glibc floor mismatch | Record minimum glibc; test on oldest claimed distro |
| musl static/runtime differences | Build and test on musl; record limitations |
| Windows MSVC runtime | Document required runtime; test clean Windows install |
| Wrong binary selection | Loader diagnostics and negative tests |
| Emulated tests | Do not claim a target tested only under emulation unless explicitly labeled |
| Node version drift | Test oldest and newest claimed versions |

## 10. Support declaration

The published support table must state:

- Node-API level.
- Tested Node.js versions.
- OS/CPU/libc matrix.
- Minimum OS or glibc floor.
- Which targets are blocking in CI.
- That alternate runtimes are untested.
- That no target is claimed without a clean-install runtime test.

“All platforms” and “all Node versions” are prohibited.
