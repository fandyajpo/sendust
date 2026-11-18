# Publication checklist and authorization (P17-003)

**Status:** prepared — no publication is authorized and no publish command has
run. The authorization table in section 6 is empty until the project owner signs
it.
**Date:** 2026-11-18.
**Candidate:** `0.1.0-rc.1`, verified in the [RC verification record](../qa/rc-verification.md).

## 1. Publication inputs

| Item | Value |
| --- | --- |
| Registry | `https://registry.npmjs.org` |
| Packages | five platform packages plus `@sendust/ts`, six in total |
| Platform packages | `@sendust/ts-linux-x64-gnu`, `@sendust/ts-linux-arm64-gnu`, `@sendust/ts-macos-arm64`, `@sendust/ts-macos-x64`, `@sendust/ts-windows-x64` |
| Versions | all `0.1.0-rc.1`; the platform manifests are generated from `ts/package.json` by `scripts/pack-platform.mjs`, so they cannot disagree |
| Facade tarball | `artifacts/publish/sendust-ts-0.1.0-rc.1.tgz` |
| Facade SHA-256 | `9470aa8ab9321d38438c8b10662bb93e622349a28c158a45d786e2af9b9b5873` |
| Platform tarballs | `artifacts/release/0.1.0-rc.1/packages/` and `PLATFORM-PACKAGES.sha256` |
| Native artifact hashes | the five hashes in the [PHASE-16 record](../gates/phase-16.md) |
| Provenance and inventory | `provenance.json`, `dependencies.json`, `SHA256SUMS` in `artifacts/release/0.1.0-rc.1/` |
| Release notes | [0.1.0-rc.1](../releases/0.1.0-rc.1.md) |
| Support matrix | [platform matrix](../qa/platform-matrix.md) and release notes |
| Licence | `MIT OR Apache-2.0`, in every manifest |
| npm dist-tag | `next` — a pre-release must never move `latest` |

The dependency inventory stands in for a CycloneDX/SPDX SBOM. PHASE-16 recorded
that deferral explicitly; it is not silently omitted.

## 2. Pre-flight checks

| # | Check | Command or evidence | Pass condition |
| --- | --- | --- | --- |
| 1 | Secret scan | `node scripts/check-secrets.mjs` | No credential shape outside the committed TLS fixture leaf keys |
| 2 | No test or source files in the tarballs | `tar -tzf` on each tarball | Only `dist/`, `README.md`, `package.json`, and (platform packages) `sendust.node` |
| 3 | Hashes match the approved candidate | `PLATFORM-PACKAGES.sha256`, `SHA256SUMS`, `sha256sum` on the facade | Every value equals the approved set |
| 4 | Unsupported targets absent | Inventory and manifest `os`/`cpu`/`libc` | Only the five claimed targets exist; no musl, macOS x64-only extras, or Windows arm64 |
| 5 | Registry scope | npm account owns the `@sendust` scope | Confirmed in the npm UI; 2FA enabled |
| 6 | Release notes current | [0.1.0-rc.1](../releases/0.1.0-rc.1.md) | Install command updated for the registry; limitations and support matrix unchanged |
| 7 | Licence present | Every tarball manifest | `MIT OR Apache-2.0` plus the repository licence files |
| 8 | Rollback readiness | [release strategy](release-strategy.md) §5–§6 | Partial-publication and bad-artifact procedures understood and owned |
| 9 | No publication yet | `git grep -n "npm publish" -- scripts .github` | Only the comment in `prepare-publish.mjs` explains why the source manifest stays private |

## 3. Publication order

1. Publish the five platform packages first, from
   `artifacts/release/0.1.0-rc.1/packages/`, each with
   `npm publish <tarball> --access public --tag next`.
2. Verify each package resolves and reports the expected `os`, `cpu`, and
   `libc`.
3. Publish the facade last:
   `npm publish artifacts/publish/sendust-ts-0.1.0-rc.1.tgz --access public --tag next`.
4. Never publish the repository root package; only `ts/` is packaged (ADR-0019).

If a platform publish fails, stop and follow the partial-publication recovery in
[release strategy](release-strategy.md) §5. Do not bump the version and do not
rebuild.

## 4. Post-publication verification

| Check | Command |
| --- | --- |
| Registry metadata | `npm view @sendust/ts@0.1.0-rc.1 --json` |
| Optional dependencies | the same output: five `@sendust/ts-*` entries at `0.1.0-rc.1` |
| Platform metadata | `npm view @sendust/ts-macos-arm64@0.1.0-rc.1 os cpu libc --json` |
| Clean install | a fresh directory: `npm install @sendust/ts@0.1.0-rc.1` |
| Load and outcome contract | the installed package under Node.js, Bun, and Deno with the loopback relay; `tests/runtime/behaviour.mjs` semantics |
| Registry artifact hash | `npm pack @sendust/ts@0.1.0-rc.1` and compare with the approved facade hash |

No post-publication check contacts an external SMTP server or uses credentials.

## 5. Tag

After the registry and clean-consumer checks pass, create the annotated tag
`v0.1.0-rc.1` on the approved candidate commit. The tag is not moved afterwards.

## 6. Authorization

**No publish command may run until this table is filled in by the project owner.**

| Field | Value |
| --- | --- |
| Decision | **PENDING** — not authorized |
| Approved by | — |
| Date | — |
| Authorized packages | — |
| Authorized dist-tag | — |
| Scope of authorization | — |
| Rollback owner | — |

The authorization must name the six packages, confirm the candidate version and
hashes, and state that publication is the authorized action. A rejection must
name the blocking work; no artifact is then published.

## 7. Rollback

Rollback is deprecation plus a fixed patch, never overwriting a published
tarball. The scenarios and the partial-publication procedure are in
[release strategy](release-strategy.md) §5–§6. A published artifact that fails a
consumer check must be deprecated with a clear message and the incident recorded
with the affected versions and the final state.
