# Publication checklist and authorization (P17-003)

**Status:** complete. `0.1.0-rc.1`, `1.0.0`, `1.0.1`, `1.0.2`, `1.1.0`, `1.1.1`,
`1.2.0`, `1.2.1` and `1.3.0` are published; the current record is §5h.
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

## 5d. 1.1.0 publication record

**Published 2026-11-22** from the CI `publish` job on the `v1.1.0` candidate:
`@sendust/ts@1.1.0` and all seven platform packages at `1.1.0`, including
`linux-x64-musl` and `linux-arm64-musl`.

| Check | Result |
| --- | --- |
| Registry version | `npm view @sendust/ts dist-tags` → `latest` `1.1.0`, `next` `1.0.0` |
| Registry artifact hash | `npm pack @sendust/ts@1.1.0` → `d19b85166073e60382f40cddbb6a096208336497812536a3ee2d0d26cd70f2e5`, the approved facade value |
| Platform packages | all seven resolve `1.1.0` |
| Tag | `v1.1.0` → `dd4336b4d30a50cdc2b2f3d3fba264b470087016` |
| Facade hash origin | the CI build, not a laptop: the first dispatch was refused on a macOS hash |

The first dispatch failed at the facade hash check and published nothing; the
pin was corrected to the CI value and the second dispatch published. A clean
install on macOS arm64 added two packages — the facade and
`@sendust/ts-macos-arm64` — and reported `version 1.1.0`. The two musl packages
were verified from registry metadata and by CI, not installed by hand here.

## 5e. 1.1.1 publication record

**Published 2026-11-23** from the CI `publish` job through npm trusted
publishing — the first release with no npm token. `@sendust/ts@1.1.1` and all
seven platform packages at `1.1.1`.

| Check | Result |
| --- | --- |
| Publisher | registry metadata: `GitHub Actions`, `trustedPublisher.id: github` |
| Provenance | `attestations.provenance.predicateType: https://slsa.dev/provenance/v1` |
| Registry artifact hash | `npm pack @sendust/ts@1.1.1` → `e6a76a0fc70ed5015377ad961e83df7fac34fc829c0c6d0af5279ad82b90c0bb`, the approved value from the `Facade hash` job |
| Dist-tags | `latest` `1.1.1`, `next` `1.0.0` |
| Platform packages | all seven resolve `1.1.1` |
| Manifest | `repository.url` present, which trusted publishing requires |
| Tag | `v1.1.1` → `1f17ece033618753676d9b736152227854cc1a90` |

Two dispatches failed before this one, both at the publish job's own guard and
both before npm ran: `actions/setup-node` exports `NODE_AUTH_TOKEN` when given
`registry-url`, and a token in the environment wins over OIDC. The job no longer
passes `registry-url`. Nothing was published by the failed runs.

## 5f. 1.2.0 publication record

**Published 2026-11-23** from the CI `publish` job through npm trusted
publishing. `@sendust/ts@1.2.0` and all seven platform packages at `1.2.0`, built
from the approved candidate `421ee02`.

| Check | Result |
| --- | --- |
| Publisher | registry metadata: `GitHub Actions`, `trustedPublisher.id: github` |
| Provenance | `attestations.provenance.predicateType: https://slsa.dev/provenance/v1` |
| Registry artifact hash | `npm pack @sendust/ts@1.2.0` → `be29213daeee25a13ac034f3ec8aaad9c808bb90bc00e3da8f7155be10005334`, the approved value from the `Facade hash` job (137491718675) |
| Registry shasum | `b31be1908f703fe57e61d2be42a98752749262e4`, 42 files, 166 977 bytes unpacked |
| Dist-tags | `latest` `1.2.0`, `next` `1.0.0` |
| Platform packages | all seven resolve `1.2.0` |
| Tag | `v1.2.0` → `421ee0283b78e1461718c326c56cbc40ea722d87` |

A later publish attempt on the npm-page candidate (`4a74817`, facade
`a93f5ef0…`) stopped at `@sendust/ts-linux-x64-gnu@1.2.0 already exists;
refusing to replace it`. That is the registry working as designed: a published
version is immutable, so the metadata ships in `1.2.1` and this record keeps the
bytes that are actually on the registry. The `v1.2.0` tag was moved back to
`421ee02` for the same reason — it must identify the published source.

## 5g. 1.2.1 publication record

**Published 2026-11-24** from the CI `publish` job through npm trusted
publishing: eight packages in one run, from the approved candidate `2190bf4`.

| Check | Result |
| --- | --- |
| Publisher | registry metadata: `GitHub Actions`, `trustedPublisher.id: github` |
| Provenance | `attestations.provenance.predicateType: https://slsa.dev/provenance/v1` |
| Registry artifact hash | `npm pack @sendust/ts@1.2.1` → `1b6de9ecdca2ddbbf59e09ecb8691a0c13b0e09152921ae30c5aae102ce1375c`, the approved value from the `Facade hash` job (138178294946) |
| Registry shasum | `503f0711c94b7c8384f64efe7a28d023741aed62`, 42 files, 170 009 bytes unpacked |
| Dist-tags | `latest` `1.2.1`, `next` `1.0.0` |
| Platform packages | all seven resolve `1.2.1`; `macos-x64` appeared last, after npm finished ingesting it — its shasum matches the one the job printed |
| npm page | `homepage`, `bugs` and 18 keywords present, on the facade and on every platform package |
| Tag | `v1.2.1` → `2190bf4cebcd48d8a6b0c7a1d789f86d123d0eb` |

The one thing to remember from this run: the job logs `+ <name>@<version>` when
npm accepts a package, and the read path can still answer 404 for a version
minutes later. Check the job log first, then the registry.

Remaining for the owner: set *Publishing access → Require two-factor
authentication and disallow tokens* on the eight packages, enable **`npm
dist-tag`** on the trusted publisher if the manual tag job is to keep working,
and then revoke the old `NPM_TOKEN` secret.

## 5h. 1.3.0 publication record

**Published 2026-11-24** from the CI `publish` job through npm trusted
publishing: eight packages in one run, from the approved candidate `e39b786`, on
the pin commit `2a721ba`.

| Check | Result |
| --- | --- |
| Publisher | registry metadata: `GitHub Actions` |
| Provenance | `attestations.provenance.predicateType: https://slsa.dev/provenance/v1` on every package |
| Registry artifact hash | `npm pack @sendust/ts@1.3.0` → `1953fd075e651f2a5724366e76e30c85862184b932eeb36e1fa9a4839adccb77`, the approved value from the `Facade hash` job (138421784504) |
| Registry shasum | `5f861ed7fc4a65a371e98db9ade1b7462b595153`, 42 files, 175.0 kB unpacked |
| Dist-tags | `latest` `1.3.0`, `next` `1.0.0` |
| Platform packages | all seven resolve `1.3.0`; `windows-x64` answered on the first lookup this time, unlike `1.2.1` |
| npm page | `homepage`, `bugs.url` and 18 keywords present |
| Tag | `v1.3.0` → `e39b7862c7d4c7839a892f83445800d84f51b662` |
| A real relay | one message to `smtp.gmail.com:465` from the published package on Node.js 22.23.5: `accepted`, `250`, 4.8 s, with `SIZE 35882577` advertised ([record](../qa/real-relay-probe.md) §3) |
| A long `AUTH` line | a 1,472-octet random token reached `smtp.gmail.com`, which answered `555` at `stage: auth`; the old cap refused it locally ([record](../qa/real-relay-probe.md) §3) |
| GitHub Release | [Sendust 1.3.0](https://github.com/fandyajpo/sendust/releases/tag/v1.3.0), published 2026-11-24; the repository's only release, so GitHub marks it Latest |

No `already exists` refusal: the version was new, and the facade digest in the
tarball is the pinned one, which is the check `publish-release.mjs` performs
before it publishes anything.

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


## 8. Trusted publishing (OIDC)

**Changed 2026-11-22.** The `publish` job no longer uses a long-lived npm token.
It publishes through npm trusted publishing: GitHub mints an OIDC token for the
job, npm exchanges it for a short-lived publish credential, and a provenance
attestation is attached automatically because the repository and the packages
are public.

What the workflow does now:

- `id-token: write` on the `publish` job, no `registry-url` on `setup-node`
  (answering it makes setup-node export `NODE_AUTH_TOKEN` with its own token,
  which wins over OIDC), `NODE_AUTH_TOKEN` pinned empty at job level as a
  backstop, and a guard step that fails the run if a real value appears;
- npm pinned to `12.2.2` (trusted publishing needs `>= 11.5.1`);
- `repository.url` in the facade and in every platform manifest, because npm
  matches it against the GitHub repository for each package it publishes;
- a dispatch-only **`Facade hash`** job that builds the facade the same way and
  prints its sha256, so the pin never comes from a developer machine.

What the owner has to do once, on npmjs.com:

1. For **each of the eight packages** (`@sendust/ts` and the seven
   `@sendust/ts-*`), open *Settings → Trusted publishing → GitHub Actions* and set
   organization/user `fandyajpo`, repository `sendust`, workflow `ci.yml`, and
   allow **`npm publish`**. Enabling **`npm dist-tag`** as well lets the manual
   `dist-tag` job run without a token.
2. Complete the first successful publish within **two days**: an unvalidated
   configuration expires and has to be recreated.
3. After the first green OIDC publish, set *Settings → Publishing access →
   Require two-factor authentication and disallow tokens* on all eight packages
   and revoke the old `NPM_TOKEN` secret.

Fallbacks and failure modes:

- Without a trusted publisher the job fails with `ENEEDAUTH`; nothing is
  published, and the fix is the configuration above, not a retry.
- The manual `dist-tag` job still uses the token, because that path is the only
  one that works before the `npm dist-tag` permission is enabled.
- If OIDC has to be abandoned, re-adding
  `NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}` to the publish step restores the
  old path; the bypass-2FA deadline in January 2027 is why that is a rollback,
  not a plan.
