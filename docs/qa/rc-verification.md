# Release candidate verification (P17-001)

**Status:** complete — the candidate identity, artifacts, and evidence match the
approved PHASE-16 handoff.
**Date:** 2026-11-18.
**Candidate:** `0.1.0-rc.1`, approved at commit `a8a71e7`, verified at
`9ed653c`.
**Verifier:** agent run recorded here; the phase decision is the project owner's
(see the [PHASE-17 gate record](../gates/phase-17.md)).

## 1. Identity

| Item | Value | Check |
| --- | --- | --- |
| Version | `0.1.0-rc.1` | `check-versions` PASS: root and facade manifests, Cargo workspace, facade constant, lockfile, and changelog agree |
| Approved commit | `a8a71e7b613e0c56bbd48655ba5a2aedaa55afa5` | PHASE-16 record |
| Verification commit | `9ed653c53e640c84611bdf76818ba7392cece600` | HEAD during verification |
| Build inputs changed since approval | none | `git diff --stat a8a71e7..9ed653c -- ts/src crates scripts/build-addon.mjs scripts/build-artifact.mjs scripts/prepare-publish.mjs scripts/assemble-rc.mjs Cargo.lock package-lock.json Cargo.toml ts/package.json package.json` prints nothing |
| Facade tarball | `artifacts/publish/sendust-ts-0.1.0-rc.1.tgz` | — |
| Facade SHA-256 | `9470aa8ab9321d38438c8b10662bb93e622349a28c158a45d786e2af9b9b5873` | Recomputed after a fresh `npm run prepare:publish`; byte-identical to the approved value |
| Facade size | 17 019 bytes, 33 entries | `tar -tzf` listing |

The commits after `a8a71e7` are documentation, test, and check changes. The
facade tarball and the five native artifacts are therefore the build of the
approved source; a fresh pack from the same tree reproduces the approved hash
exactly.

## 2. Native artifacts

The per-target set is the one the `Release candidate bundle` CI job produced from
`a8a71e7` and uploaded as `release-candidate`. Those hashes, not a rebuild, are
what P17 verifies; different hosts link different bytes, which the
[reproducible build](../engineering/reproducible-build.md) note has documented
since P16.

| Target | CI SHA-256 (approved set) |
| --- | --- |
| `linux-x64-gnu` | `7527891da22c7b4e672c1db6635b3a597f2c1ee03d770ae5024045ddf1bf0548` |
| `linux-arm64-gnu` | `bf90530b39ae66236d358c3e2311ef386f9c357aa56e5900a42fcc0f9c776716` |
| `macos-arm64` | `8da616f24ec1d2c675774641e7bbbb7976091bd552a52e8834a3aa507f007a55` |
| `macos-x64` | `f561cbf5e6373afbaff3a81f21661c9f8d376d14c0c48ccbc7755ace6ae3e215` |
| `windows-x64` | `afcd488212a89bb88b57e830f2b6c4617d9cac81ac1ccf7341b16c44454385de` |

Local rebuild on macOS arm64 at the verification commit:
`a39b370a6c28ef96a70c37bd133d815e6801629d858a079234fd49909221eb86`, the same
hash the P16 record captured for a local rebuild and different from the CI
runner's `8da616f2…` binary. The immutable reference for publication is the CI
set above.

## 3. Artifact contents

The published facade contains exactly what the metadata contract allows:

- `dist/` — ESM and CommonJS entry points, transports, loaders, errors, limits,
  results, and types, with declarations (33 files including `package.json` and
  `README.md`);
- `package.json` — `"private": false`, `"name": "@sendust/ts"`, version
  `0.1.0-rc.1`, dual licence, `engines.node >= 22.23.5`, and five
  `optionalDependencies` (`@sendust/ts-linux-x64-gnu`, `-linux-arm64-gnu`,
  `-macos-x64`, `-macos-arm64`, `-windows-x64`) all pinned to `0.1.0-rc.1`;
- no bundled `sendust.node`, no `src/`, no `test/`, no `.env`, no source maps,
  no fixtures, no `target/`.

`prepare-publish` prints the declared packages on every run, so the manifest
cannot drift from the loader's own platform map.

## 4. Evidence completeness

| Item | State | Evidence |
| --- | --- | --- |
| Prerequisite gates | PHASE-11, PHASE-12, PHASE-13, PHASE-14, PHASE-15 approved | [gates](../gates/) |
| Candidate gate | PHASE-16 approved 2026-11-18 | [phase-16.md](../gates/phase-16.md) |
| Candidate CI | 14/14 jobs green on `e903fce`, including `Release candidate bundle` | GitHub Actions |
| Release checklist | 14 items PASS | [release checklist](../engineering/release-checklist.md) |
| Provenance and inventory | `provenance.json`, `dependencies.json`, `SHA256SUMS` | `artifacts/release/0.1.0-rc.1/` |
| Publication action | none | `git grep -n "npm publish" -- scripts .github` finds only a comment explaining why the source manifest stays private |

## 5. Critical smoke from the candidate artifacts

All runs are loopback only: no external network, no relay credentials.
Commands are the ones in the [release checklist](../engineering/release-checklist.md)
section 1a and the [platform validation](platform-validation.md) procedure.

| Check | Result |
| --- | --- |
| Clean-consumer install of the packed facade, published layout | PASS — `scripts/verify-platform.mjs`: npm installed only `@sendust/ts-macos-arm64` beside the facade |
| Node.js ESM and CommonJS import through the installed package | PASS |
| Bun import through the installed package | PASS |
| Deno import through the installed package (`--allow-ffi --allow-read --allow-net`) | PASS |
| Missing platform package | PASS — typed `CONFIG_NATIVE_UNAVAILABLE` |
| Corrupt addon | PASS — typed `CONFIG_NATIVE_UNAVAILABLE` |
| No platform package installed | PASS — typed `CONFIG_NATIVE_UNAVAILABLE` |
| Behaviour harness, Node.js 22.23.5 | PASS — 16/16 |
| Behaviour harness, Bun | PASS — 16/16 |
| Behaviour harness, Deno 2.9.8 | PASS — 16/16 |
| Facade runtime smoke, Node.js | PASS — 6/6 |
| Quick-start documentation sample | PASS — accepted send, close, `TRANSPORT_CLOSED` after close |
| Facade unit and contract suite | PASS — 60 tests in 6 files |
| Rust workspace suites | PASS — `cargo test --workspace --locked` |
| Performance harnesses | PASS — recorded in the [PHASE-16 record](../gates/phase-16.md) |

### Coverage of the P17-001 criteria

| Criterion | Covered by |
| --- | --- |
| Accepted send | behaviour harness "accepted"; quick-start sample |
| Post-DATA `unknown` | behaviour harness "unknown status / recipient / response" |
| `BUSY` with no dial | behaviour harness "busy" and "busy connections" (1 connection); lifecycle probe "50 concurrent sends, 0 extra connections" |
| Close | behaviour harness "close aborts the send", "closed state", "send after close"; lifecycle probe idle-close and descriptor bound |
| TLS verification failure | behaviour harness "tls fails closed" (`TLS_CERTIFICATE`) |
| Attachment opt-in | `ts/test/transport.test.ts` "rejects a path attachment without filesystem opt-in" (`CONFIG_FILE_ACCESS_DISABLED`); core `rejects_file_attachments_without_opt_in` |
| Cap rejection | behaviour harness "attachment cap" (`LIMIT_ATTACHMENT_BYTES`); `ts/test/limits.test.ts` |

## 6. Findings

| Severity | Finding | State |
| --- | --- | --- |
| Low | The staged publish manifest keeps the repository's `scripts` and `devDependencies`, which name files (`ts/scripts/*`, `src/`, `test/`) that the tarball does not ship. Consumers are unaffected: npm never installs a dependency's `devDependencies` and never runs its `scripts`. | Deferred — stripping them changes the tarball hash, so it requires a new candidate version rather than an edit to the approved one |
| Low | Local assembly now records commit `9ed653c` in `provenance.json` rather than the approved `a8a71e7`, because the assembly was re-run at HEAD. Build inputs are identical (section 1); the approved artifact set is unchanged. | Explained — no action |
| Low | The local Bun is 1.3.14, not the pinned 1.4.4. The CI behaviour suite runs 1.4.4 per target, which is the support claim; the local run is corroboration only. | Explained — no action |

No high- or medium-severity finding is open, and no identity mismatch was found.

## 7. Verdict

The candidate version, commit, facade hash, and five artifact hashes match the
approved PHASE-16 handoff; the facade tarball reproduces byte for byte from the
same source; the published layout installs and refuses missing, corrupt, and
absent binaries with typed errors; and the outcome contract passes on every
claimed runtime. P17-001 is complete, and P17-002 is unblocked.
