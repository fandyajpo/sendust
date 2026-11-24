# Release gate — 1.2.1

**Decision:** APPROVED — project owner, 2026-11-24. The owner asked for the npm
page to be fixed; `1.2.0` is published and immutable, so the metadata ships as a
patch.
**Release:** `@sendust/ts@1.2.1`, dist-tag `latest`.
**Candidate commit:** `2190bf4cebcd48d8a6b0c7a1d789f86d123d0deb`, the `release:
prepare 1.2.1` commit, and the commit the `v1.2.1` tag points at.
**Related:** [release notes](../releases/1.2.1.md),
[1.2.0 gate record](release-1.2.0.md),
[package metadata contract](../qa/package-metadata-contract.md),
[publication checklist](../engineering/publication-checklist.md).

## 1. Scope

Metadata only: `homepage`, `bugs` and `keywords` in the published manifest, a
`ts/README.md` written as the package page, and the same project fields copied
into the seven platform packages.

Patch is the correct bump: no API, result, code, limit or default changed, and no
message byte changes.

## 2. Why this release exists

`1.2.0` is on the registry (facade `be29213d…`, published 2026-11-23) and its npm
page has no homepage link and no keywords, so it is invisible to npm search and
points only at the repository. A published version cannot be replaced, so the fix
is a patch.

## 3. Candidate checklist

| Check | Evidence |
| --- | --- |
| Owner approval recorded | This record, 2026-11-24 |
| Contract unchanged | `public-api.md`, `error-model.md`, `types.ts` untouched |
| Rust suite | `cargo test --workspace --all-targets` green |
| Rust lint and format | `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check` |
| TypeScript | `tsc`, Biome, 76 Vitest tests |
| Documentation guards | `npm run check` PASS at 1.2.1 |
| Manifest fields | read back out of `artifacts/publish/sendust-ts-1.2.1.tgz`: `private false`, seven optional dependencies, `homepage`, `bugs`, 18 keywords |
| Platform manifest | a packed `@sendust/ts-macos-arm64` carries `repository`, `homepage`, `bugs`, `keywords` |
| Native artifacts | CI `Native artifact` and `Native artifact (musl)` jobs on the candidate commit |
| Facade hash | `1b6de9ec…` from the `Facade hash` dispatch on the candidate commit (`1b6de9ec…`, job 138178294946); never from a laptop |

## 4. Binary verification

`release/1.2.1/approved.json` pins the approved commit, the dist-tag `latest`, the
facade hash `1b6de9ecdca2ddbbf59e09ecb8691a0c13b0e09152921ae30c5aae102ce1375c`
produced by the `Facade hash` job on that commit (job 138178294946), and all seven
platform entries. No byte-reproducibility claim: each target is identified by the
approval bundle's own checksums, as in `1.1.1`. `publish-release.mjs` refuses a
facade whose digest differs from the pinned value
([reproducible build](../engineering/reproducible-build.md) §7).

## 5. Publication

Not executed. The `publish` job publishes `@sendust/ts@1.2.1` and the seven
platform packages over npm trusted publishing (OIDC), with no npm token. After it
runs, this section records the result, the tag and the post-publication `npm pack`
digest.
