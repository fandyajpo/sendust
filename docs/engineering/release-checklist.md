# Release checklist and rollback plan

**Status:** P16-004. Used when assembling a release candidate (P16) and again
when publication is considered (P17). Assembly never publishes.

## 1. Candidate checklist

Every item names its owner, the evidence to attach, and the pass condition. No
item may rely on unpublished or remembered evidence.

| # | Check | Owner | Evidence | Pass condition |
| --- | --- | --- | --- | --- |
| 1 | Version agreement | Release | `node scripts/check-versions.mjs` | One version across the root and facade manifests, the workspace, the facade constant, the lockfile, and the changelog |
| 2 | Clean build | CI | Rust workspace job | `cargo fmt --check`, clippy `-D warnings`, unit and integration suites pass on the commit |
| 3 | TypeScript contract | CI | Node 22/24/26 jobs | Lint, format, typecheck, and the 61-test suite pass |
| 4 | Runtime smokes and behaviour | CI | Native binding and artifact jobs | Binding, facade, behaviour harness, and doc sample pass on Node.js, Bun, and Deno |
| 5 | Security gate | Security | PHASE-11 record, `check-secrets`, redaction tests | No open high-severity finding; no credential shape outside fixtures |
| 6 | Performance report | Performance | PHASE-13 record | Every budget met; no unexplained regression |
| 7 | Distribution matrix | Distribution | PHASE-14 record | All five targets build, load, and pass clean-consumer selection |
| 8 | Documentation accuracy | Documentation | PHASE-15 record, `check-docs`, `check-doc-claims`, doc sample | Links valid; claims, codes, and numbers match the code |
| 9 | Artifact checksums | Release | `npm run assemble:rc` → `SHA256SUMS` | One hash per artifact, recorded in `provenance.json` |
| 10 | Dependency inventory | Release | `dependencies.json` from the assembler | Rust and npm inventories present; no unexpected package |
| 11 | Licence | Release | `LICENSE-MIT`, `LICENSE-APACHE`, manifest fields | Dual licence present in both manifests and the facade tarball |
| 12 | Publish manifest | Release | `npm run prepare:publish` | Tarball declares the five platform packages, `private: false`, no bundled addon |
| 13 | No publication action | Release | `git grep -n "npm publish" -- scripts .github` returns nothing | No script or workflow publishes |
| 14 | Clean tree | Release | `provenance.json` `dirty: false` | Candidate assembled at a commit with no uncommitted changes |

## 2. Candidate versus publication

- **Candidate assembly (P16)** produces artifacts, hashes, provenance, notes,
  and this checklist. It runs no publish command.
- **Publication (P17)** verifies the candidate hash-by-hash, records a named
  approver, publishes the platform packages first and then the facade, and
  verifies the registry metadata and a clean install.
- A candidate that fails any checklist item is not handed to P17.

## 3. Rollback plan

Publication is not atomic, so rollback is about an honest, documented state, not
about rewriting the registry.

| Scenario | Response |
| --- | --- |
| Bad native binary | Deprecate the version with a clear message; publish a fixed patch; keep the old tarball for audit |
| Contract bug | Deprecate; publish a patch, or a major version if status semantics or error codes change |
| Security issue | Deprecate; issue an advisory; publish a fixed patch; consider unpublish only within registry policy |
| Install breakage (metadata) | Deprecate; fix the metadata; publish a patch; never overwrite the tarball |
| Partial publication | Complete the missing packages from the unchanged artifacts, or deprecate the root version if it is already broken |

Rules that apply to every scenario:

- Never bump the version to fix a publication gap; publish the missing pieces
  from the same version.
- Never replace a published tarball or a native artifact under the same version.
- Never move a tag after publication.
- Record the incident, the affected versions, and the final state.
- Consumers already on the bad version need a communication path: the release
  notes and the npm deprecation message are the minimum.

## 4. Evidence collection

For the RC handoff (P16-005), attach:

- `artifacts/release/<version>/provenance.json`, `dependencies.json`,
  `SHA256SUMS`;
- the CI run URL for the candidate commit;
- links to the PHASE-11 to PHASE-15 gate records;
- the release notes draft;
- this checklist, filled with pass conditions and evidence links.

## 5. Related reading

- [Release strategy](release-strategy.md) — publication, recovery, and yanking.
- [Reproducible build](reproducible-build.md) — inputs and provenance.
- [Versioning policy](versioning-policy.md) — tags and changelog.
