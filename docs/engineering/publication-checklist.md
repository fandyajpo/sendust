# Publication checklist and authorization (P17-003)

**Status:** complete. `0.1.0-rc.1`, `1.0.0`, `1.0.1`, and `1.0.2` are published;
the current record is §5c.
**Date:** 2026-11-18.
**Original candidate:** `0.1.0-rc.1`, verified in the
[RC verification record](../qa/rc-verification.md).

## 1. Publication inputs

| Item | Value |
| --- | --- |
| Registry | `https://registry.npmjs.org` |
| npm account | `fanvercel` (signed in on the build host) |
| npm scope | `@sendust` — exists; `npm org ls sendust` reports `fanvercel - owner` |
| Packages | five platform packages plus `@sendust/ts`, six in total |
| Platform packages | `@sendust/ts-linux-x64-gnu`, `@sendust/ts-linux-arm64-gnu`, `@sendust/ts-macos-arm64`, `@sendust/ts-macos-x64`, `@sendust/ts-windows-x64` |
| Versions | all `0.1.0-rc.1`; the platform manifests are generated from `ts/package.json` by `scripts/pack-platform.mjs` |
| Approved hashes | `release/0.1.0-rc.1/approved.json` — facade hash, the five artifact hashes, and the `byteReproducible` target list |
| Facade tarball | produced by `prepare:publish`; must reproduce `9470aa8a…` |
| Platform tarballs | produced by `pack:platform`; each must contain the approved target binary |
| Provenance and inventory | `provenance.json`, `dependencies.json`, `SHA256SUMS`, `platform-packages.json`, `PLATFORM-PACKAGES.sha256` |
| Release notes | [0.1.0-rc.1](../releases/0.1.0-rc.1.md) |
| Support matrix | [platform matrix](../qa/platform-matrix.md) and release notes |
| Licence | `MIT OR Apache-2.0`, in every manifest |
| npm dist-tag | `next`, set explicitly. npm also assigns `latest` on a package's first publish and the registry refuses to remove it, so `latest` equals the RC until the first stable release moves it |

The dependency inventory stands in for a CycloneDX/SPDX SBOM; the PHASE-16 record
documents that deferral.

## 2. Pre-flight checks

| # | Check | Evidence | Pass condition |
| --- | --- | --- | --- |
| 1 | Secret scan | `node scripts/check-secrets.mjs` | No credential shape outside the committed TLS fixture leaf keys |
| 2 | No test or source files in the tarballs | `tar -tzf` on each | Only `dist/`, `README.md`, `package.json`, and, for platform packages, `sendust.node` |
| 3 | Hashes match the approved candidate | `scripts/publish-release.mjs` | Facade equals `approved.facade.sha256`; the bundle's `provenance.json` names the approved commit or a later one whose build inputs are unchanged; each tarball contains the binary the bundle's `SHA256SUMS` records; each `byteReproducible` target also equals its approved hash |
| 4 | Unsupported targets absent | Inventory, `os`/`cpu`/`libc` | Only the five claimed targets exist |
| 5 | Registry scope | npm UI | An organization named `sendust` exists with `fanvercel` as owner |
| 6 | Publish credentials | GitHub repository secret | `NPM_TOKEN` is set, granular, scoped to `@sendust` read-and-write, with a short expiry and bypass-2FA enabled so CI can publish without an interactive OTP |
| 7 | Release notes current | [0.1.0-rc.1](../releases/0.1.0-rc.1.md) | Install command updated for the registry; limitations and support matrix unchanged |
| 8 | Licence present | Every tarball manifest | `MIT OR Apache-2.0` plus the repository licence files |
| 9 | Rollback readiness | [release strategy](release-strategy.md) §5–§6 | Partial-publication and bad-artifact procedures understood and owned |
| 10 | No publication yet | `git grep -n "npm publish" -- scripts .github` | Only the comment in `prepare-publish.mjs` explains why the source manifest stays private |

Checks 5 and 6 are the owner's, and both are in place: the organization exists,
and the owner reports the repository secret is set. The publish job's `npm
whoami` step is the check that the token is accepted before anything is
published.

## 3. Execution path

Publication runs from CI, from the `release-candidate` bundle that the same
workflow run produced and verified, not from a developer machine
([release strategy](release-strategy.md) §10).

1. Done on 2026-11-18: the project owner created the `sendust` organization on
   npmjs.com with `fanvercel` as an owner.
2. Done by the project owner (reported 2026-11-18): the repository secret
   `NPM_TOKEN` is a granular token with read-and-write access to the `@sendust`
   scope, **Bypass 2FA** enabled, a seven-day expiry, and no IP allow-list
   (GitHub runners are dynamic). Bypass 2FA is deprecated for direct publishing
   in January 2027, so the token is a short-lived release credential: revoke it
   after the publish, and move to trusted publishing (OIDC) or a stage-only token
   with maintainer promotion before then. A stage-only token cannot publish
   directly: CI would stage the six versions and a maintainer would promote each
   with an OTP.
3. Run the `CI` workflow manually with **publish** checked
   (`gh workflow run ci.yml -f publish=true`, or *Run workflow* in the Actions
   UI). The `publish` job downloads the candidate bundle, rebuilds the facade,
   and runs `scripts/publish-release.mjs --tag next`, which verifies the approved
   build inputs, the facade hash, the addon inside each platform tarball against
   the bundle checksums, and byte equality for the byte-reproducible targets,
   then publishes the five platform packages and then `@sendust/ts`.
4. A package that already exists at `0.1.0-rc.1` aborts the run; nothing is
   replaced.

Manual publication is not an approved alternative: the local machine holds one
target, and a partial release is an incident.

## 4. Post-publication verification

| Check | Command |
| --- | --- |
| Registry metadata | `npm view @sendust/ts@0.1.0-rc.1 --json` |
| Optional dependencies | the same output: five `@sendust/ts-*` entries at `0.1.0-rc.1` |
| Platform metadata | `npm view @sendust/ts-macos-arm64@0.1.0-rc.1 os cpu libc --json` |
| Dist-tag | `npm view @sendust/ts dist-tags` shows `next`, not `latest` |
| Clean install | a fresh directory: `npm install @sendust/ts@0.1.0-rc.1` |
| Load and outcome contract | the installed package on Node.js, Bun, and Deno against the loopback relay |
| Registry artifact hash | `npm pack @sendust/ts@0.1.0-rc.1` and compare with `9470aa8a…` |

No post-publication check contacts an external SMTP server or uses credentials.

Executed on 2026-11-18: the facade tarball from the registry reproduces
`9470aa8a…`, the five optional dependencies are pinned to `0.1.0-rc.1`, platform
metadata carries the expected `os`/`cpu`/`libc`, a clean install pulls only the
facade and the local platform package, and the behaviour contract passes 16/16 on
Node.js, Bun, and Deno. The [PHASE-17 record](../gates/phase-17.md) section 4
holds the full table.

## 5. Tag

The annotated tag `v0.1.0-rc.1` was created on the approved candidate commit on
2026-11-18 and pushed (`refs/tags/v0.1.0-rc.1` resolves to
`a8a71e7b613e0c56bbd48655ba5a2aedaa55afa5`). The tag is not moved afterwards.

The annotated tag `v1.0.0` was created on `f1559344b8f6bc67a87300244f895f19a9cf5d3f`
and pushed on 2026-11-19 (`refs/tags/v1.0.0`).

The annotated tag `v1.0.1` was created on `cad80fd33afcd92914d1c6d3d91db01749abcb9e`
and pushed on 2026-11-21 (`refs/tags/v1.0.1`, tag object
`8432db90cfdef24ea0ea160bab44e805de742159`). It was first created at `377aa58` and
moved to `cad80fd` before anything was published, because the earlier commit's
tests still encoded the single-session behaviour; the
[gate record](../gates/release-1.0.1.md) holds that history.

## 5a. 1.0.0 publication record

**Published 2026-11-19** from the CI `publish` job: `@sendust/ts@1.0.0` and the
five platform packages at `1.0.0`, with `optionalDependencies` pinned to the same
version.

| Check | Result |
| --- | --- |
| Registry version | `npm view @sendust/ts@1.0.0 version` → `1.0.0` |
| Platform packages | all five at `1.0.0` |
| Optional dependencies | the five `@sendust/ts-*` entries at `1.0.0` |
| Clean install | fresh directory: `added 2 packages` (facade and `@sendust/ts-macos-arm64`) |
| Facade import | `version` → `1.0.0` |
| Registry artifact hash | `npm pack @sendust/ts@1.0.0` → `4b0290741459729ae16525551fe96e9b97b46551e2cce8833ef8973fb908ae1f`, the approved value |

**Dist-tag incident.** The `publish` job hardcoded `--tag next`, so `1.0.0` was
published under `next` while `latest` still resolved to `0.1.0-rc.1`. The
workflow now reads `distTag` from `release/<version>/approved.json`, and a
manual `dist-tag` job was added: a published version is immutable, so the tag is
moved with `npm dist-tag add`, never by republishing. The owner ran that job on
2026-11-19: `latest` and `next` both resolve `1.0.0`, and a bare
`npm install @sendust/ts` installs `1.0.0`.

## 5b. 1.0.1 publication record

**Published 2026-11-21** from the CI `publish` job on the `v1.0.1` candidate:
`@sendust/ts@1.0.1` and the five platform packages at `1.0.1`, with
`optionalDependencies` pinned to the same version.

| Check | Result |
| --- | --- |
| Registry version | `npm view @sendust/ts@1.0.1 version` → `1.0.1` |
| Dist-tag | `npm view @sendust/ts dist-tags` → `latest` `1.0.1`; `next` `1.0.0` |
| Platform packages | all five at `1.0.1` |
| Optional dependencies | the five `@sendust/ts-*` entries at `1.0.1` |
| Clean install | fresh directory: `added 2 packages`, then `version` → `1.0.1` |
| Registry artifact hash | `npm pack @sendust/ts@1.0.1` → `004eb54a03656c1a4311b6b0d91f6f7efa6e3b7331992cdd94674e002ac1f56f`, the approved facade value |

The published facade tarball is byte-identical to the approved candidate, so the
release carries no caveat about which build went up. The `next` tag was left at
`1.0.0`: a published version's tag is moved with the manual `dist-tag` job when a
pre-release channel is wanted, and a bare `npm install` reads `latest`.

## 5c. 1.0.2 publication record

**Published 2026-11-21** from the CI `publish` job on the `v1.0.2` candidate:
`@sendust/ts@1.0.2` and the five platform packages at `1.0.2`, with
`optionalDependencies` pinned to the same version.

| Check | Result |
| --- | --- |
| Registry version | `npm view @sendust/ts dist-tags` → `latest` `1.0.2`, `next` `1.0.0` |
| Registry artifact hash | `npm pack @sendust/ts@1.0.2` → `90ce59433dc864e49dbe3f940f50d3a40788b808c8473216d9628507d48bd20a`, the approved facade value |
| Tag | `v1.0.2` → `bc72a6c28edd7464fde3107579fa4fb5fbce5b7c` |
| Platform packages | all five at `1.0.2`, per the approved manifest |

The published tarball reproduces the approved build byte for byte. A fresh
clean-install run of `1.0.2` on the publishing machine is outstanding; the
platform packages and optional dependency pins were checked from the registry
metadata.

## 6. Authorization

**APPROVED by the project owner on 2026-11-18.**

| Field | Value |
| --- | --- |
| Decision | APPROVED |
| Approved by | project owner |
| Date | 2026-11-18 |
| Authorized packages | `@sendust/ts-linux-x64-gnu`, `@sendust/ts-linux-arm64-gnu`, `@sendust/ts-macos-arm64`, `@sendust/ts-macos-x64`, `@sendust/ts-windows-x64`, `@sendust/ts` |
| Authorized version | `0.1.0-rc.1` |
| Authorized dist-tag | `next` |
| Scope of authorization | the six packages above, published from the CI `publish` job from a CI build of the approved build inputs, with the facade and the byte-reproducible targets matching their approved hashes and platform packages first |
| Rollback owner | project owner |

The authorization covers exactly this candidate. A hash mismatch in the facade or
a byte-reproducible target, a failed consumer check before publication, or any
later change to the version reopens this gate. A Windows DLL that links to
different bytes is not a hash mismatch under this policy: Windows is identified by
the approved build inputs and the bundle's checksums, as
[reproducible build](reproducible-build.md) records. The run completed on
2026-11-18 and every post-publication check passed, so this authorization is
spent: the `@sendust` organization exists, the owner reports the `NPM_TOKEN`
secret is set, and the `Rust workspace` flake that failed the previous manual
run is fixed in the PHASE-17 record. The dispatches for `1.0.0` and `1.0.1` used
the same path and are recorded in §5a and §5b.

## 7. Rollback

Rollback is deprecation plus a fixed patch, never overwriting a published
tarball. Scenarios and the partial-publication procedure are in
[release strategy](release-strategy.md) §5–§6. A published artifact that fails a
consumer check must be deprecated with a clear message, and the incident recorded
with the affected versions and the final state.
