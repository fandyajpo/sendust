# Reproducible build and artifact provenance

**Status:** P16-002. Defines the build inputs, the assembly recipe, the
determinism evidence, and the provenance record for a release candidate.

## 1. Deterministic inputs

| Input | Where it is pinned |
| --- | --- |
| Rust toolchain | `rust-toolchain.toml` (1.100.0, with rustfmt and clippy) |
| Rust dependencies | `Cargo.lock`, committed; `--locked` on every build |
| Node dependencies | `package-lock.json`, committed; `npm ci` |
| Build tooling | `ts/package.json` devDependencies: TypeScript 7.0.2, Biome 2.6.7, Vitest 5.1.0 |
| Node.js, Bun, Deno | [toolchain baseline](toolchain-baseline.md) |
| Node-API level | `napi = { features = ["napi8", "async"] }` in `Cargo.toml` |
| TLS | `rustls` with the `ring` provider and `webpki-roots`, no `native-tls`, no alternate verifier |
| Release profile | `lto = "thin"`, `codegen-units = 1`, `strip = "symbols"` in `Cargo.toml` |
| Target | The runner's own OS/CPU; no cross-compilation |

`scripts/check-architecture.mjs` enforces the dependency, feature, and module
rules, so a candidate cannot be built with a different TLS feature set.

## 2. Recipe

```text
npm ci                       # locked Node dependencies
npm run build:native         # cargo build --release --locked, staged to ts/sendust.node
npm run build --workspace ts # ESM and CommonJS output
npm run build:artifact       # artifacts/sendust-<platform>-<arch>.node
npm run prepare:publish      # facade tarball with platform optionalDependencies
npm run assemble:rc          # provenance, dependency inventory, SHA256SUMS
```

The full `npm run verify` runs the same steps plus the tests, runtime smokes, doc
sample, and clean-consumer selection check.

No step downloads code at build time beyond the locked dependencies, and none
requires a secret, a relay, or network access after `npm ci`.

## 3. Determinism evidence

On macOS 15 / Apple M2, Rust 1.100.0, Node.js 22.23.5, a clean rebuild of
`sendust-napi` (after `cargo clean -p sendust-napi`) produced a byte-identical
addon:

```text
first  e7d429ac82080d8ed4c99a0cebe6fdb68b8d0c72fef0cf344af90829ed623bc2
second e7d429ac82080d8ed4c99a0cebe6fdb68b8d0c72fef0cf344af90829ed623bc2
```

So the inputs and flags are sufficient to reproduce the artifact on this host and
toolchain. The evidence is a hash comparison, not a claim that every host
produces the same bytes: linkers and platform SDKs can embed host-specific
metadata. The candidate is therefore **built once in CI**, its hashes are frozen
in provenance, and those hashes — not a rebuild — are what publication
verifies.

The CI set was checked across runs while P17 verified the candidate. `linux-x64-gnu`,
`linux-arm64-gnu`, `macos-arm64`, and `macos-x64` reproduced byte for byte in
every run, including runs at later commits whose only changes were documentation
and tests, so they are byte-identity checked at publication. The Windows DLL did
not: two workflow runs of the same commit produced `85bfa57f…` and `fe50601e…`,
and the approval run produced `afcd4882…`. Windows linking is therefore recorded
as nondeterministic, and publication holds it to the approved commit and the
bundle's own `SHA256SUMS` instead of a frozen hash from another run. The list is
data, not a hidden script constant: `release/<version>/approved.json` names the
`byteReproducible` targets.

## 4. Provenance record

`npm run assemble:rc` writes `artifacts/release/<version>/`. A dedicated CI job
(`Release candidate bundle`) downloads all five artifacts and runs it with
`--require-all`, so the candidate carries one complete hash set; `--allow-stale`
is used there only because downloaded artifacts legitimately predate that job's
checkout. Every artifact job runs the assembler against its own fresh build,
where the staleness guard is active.

| File | Contents |
| --- | --- |
| `provenance.json` | package name and version, source commit, dirty flag, toolchain versions (rustc, cargo, Node, npm, TypeScript, Biome, Vitest, Bun, Deno when present), and each artifact's target, byte size, and SHA-256 |
| `dependencies.json` | Rust crates from `Cargo.lock` and npm packages from `package-lock.json` |
| `SHA256SUMS` | `<hash>  sendust-<platform>-<arch>.node` per artifact |

The assembler also prints the checksum lines, so a CI log carries the same set
the bundle file does.

`dirty` is `true` when the working tree has uncommitted changes; a release
candidate must be assembled from a clean tree at the commit the gate record names.

## 5. Negative checks

The assembler refuses to produce evidence when:

- an artifact's platform key is not one of the five supported targets;
- no artifact is present (unless `--allow-empty`, used only by guarded runs);
- `scripts/check-secrets.mjs` fails, so a credential cannot reach the artifacts
  or the evidence.

A missing checksum, a missing artifact, or a failed secret scan blocks the
candidate at P16-005.

## 6. What is not claimed

- Byte-identical output across different hosts or platform SDKs.
- Byte-identical Windows output across CI runs; the DLL is identified by the
  approved source commit and the bundle's own checksums instead.
- Signed or notarized artifacts; macOS signing and notarization are not part of
  the MVP.
- A formal SBOM standard: the inventory is a flat, reviewable list, not CycloneDX
  or SPDX.
- Provenance for a published registry release: that is P17, which verifies the
  frozen hashes rather than rebuilding.

## 7. Related reading

- [Versioning policy](versioning-policy.md) — version sources and tags.
- [Release strategy](release-strategy.md) — publication and rollback.
- [Platform validation](../qa/platform-validation.md) — the artifact set.
