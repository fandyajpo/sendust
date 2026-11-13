# Release Strategy

**Applies to:** P16, P17, P18.  
**Related:** [distribution strategy](distribution-strategy.md), [quality gates](../planning/quality-gates.md), [ADR-0011](../architecture/decisions/0011-platform-distribution.md).

## 1. Principles

1. A release is a production change, not a build preview.
2. Artifacts are immutable once published. Never replace binaries under an existing version.
3. Every release is reproducible, provenance-tracked, and evidence-linked.
4. Publication is authorized, not automatic on merge.
5. Partial publication is an incident with a documented recovery path.
6. Support claims match CI, not aspiration.

## 2. Versioning

- Semantic Versioning: `MAJOR.MINOR.PATCH`.
- Pre-release: `X.Y.Z-rc.N` or `X.Y.Z-next.N`.
- Git tag: `vX.Y.Z` for stable, `vX.Y.Z-rc.N` for candidates.
- Version source of truth: the root package manifest and workspace metadata must agree; P16-001 fixes the mechanism.
- Changelog categories: Added, Changed, Fixed, Security, Deprecated, Removed.

### Compatibility rules

| Change | Version |
| --- | --- |
| Add optional option or error code | Minor |
| Add new platform target | Minor |
| Change status semantics or remove/rename a code | Major |
| Change a default timeout or cap downward | Major |
| Raise a hard cap | Major |
| Loosen a hard cap or add a feature | Minor |
| Fix behavior without contract change | Patch |
| Security fix | Patch or minor, with advisory |

Delivery semantics (`accepted` = relay acceptance), no-retry behavior, and lifecycle rules are part of the contract. Changing them is a major version.

## 3. Release candidate process

1. All prerequisite gates approved: QA, security, performance, distribution, documentation.
2. Version bumped and changelog updated.
3. Reproducible build assembled with locked dependencies and pinned toolchain.
4. Artifacts collected, checksummed, and inventoried.
5. SBOM and provenance generated.
6. Release notes drafted with limitations and support matrix.
7. Checklist completed with evidence links.
8. RC package handed to approval; **no publish command runs**.

## 4. Publication process

1. Verify RC identity: version, commit, hashes match the approved candidate.
2. Confirm artifact completeness for every declared target.
3. Confirm root and platform package versions are identical.
4. Confirm provenance and SBOM.
5. Publish platform packages first, then the root package, using the documented napi-rs flow.
6. Verify registry metadata:
   - `npm view sendust@<version> --json`
   - optional dependency versions,
   - platform package `os`, `cpu`, `libc`,
   - tarball contents,
   - provenance.
7. Install from the registry in clean projects on representative targets.
8. Run a fake-relay send and close.
9. Record the verification results with the release.

## 5. Partial publication recovery

npm publication is not atomic.

If a platform package fails after others are published:

1. Do not bump the version.
2. Do not rebuild or replace existing artifacts.
3. Inventory what was published.
4. Publish the missing platform packages from the unchanged artifacts.
5. If the root package is missing, publish the unchanged root tarball with lifecycle scripts disabled.
6. If the root is published with a missing optional dependency, publish the missing package immediately or deprecate the broken root version.
7. Document the incident and the final state.

If a published artifact is defective:

- Publish a new patch version with the fix.
- Deprecate the defective version with a clear message.
- Do not overwrite the tarball.
- If the defect is security-critical, issue an advisory and notify consumers.

## 6. Rollback and yanking

| Scenario | Action |
| --- | --- |
| Bad native binary | Deprecate version; publish fixed patch; keep old tarball for audit |
| Contract bug | Deprecate; publish patch or major depending on semantics |
| Security issue | Deprecate; advisory; fixed patch; consider unpublish only if policy allows |
| Install breakage | Deprecate; fix metadata; publish patch |
| Partial publish | Complete from unchanged artifacts or deprecate root |

Unpublish is a last resort and only within registry policy. It is not a substitute for a fixed release.

## 7. Release notes

Every release notes document includes:

- Summary and motivation.
- Supported Node.js versions and platform matrix.
- Installation instructions.
- Semantic contract: `accepted` is relay acceptance, not delivery.
- `unknown` after DATA and no automatic retry.
- `BUSY`, close, and lifecycle behavior.
- Caps and timeout defaults.
- Attachment opt-in and file restrictions.
- TLS requirements and limitations (for example, no private CA in MVP).
- Known limitations.
- Migration notes for breaking changes.
- Links to documentation, security policy, and support.

Prohibited claims:

- “Guaranteed delivery.”
- “Automatically retries.”
- “Supports all Node versions.”
- “Works on all platforms.”
- Any capability not in the [feature matrix](../planning/feature-matrix.md).

## 8. Approval

- P17-002 runs final acceptance against requirements and the feature matrix.
- P17-003 records a named approver and the publication decision.
- The approver confirms: gates pass, artifacts match, notes are accurate, rollback plan exists.
- No publication without an explicit approval record.

## 9. Post-release

- Monitor issues and security reports.
- Triage regressions within the compatibility policy.
- P18 collects feedback and evaluates post-MVP candidates.
- A post-release defect gets a regression test before the fix.
- Support-matrix changes require a new release.

## 10. What is explicitly out of scope for release engineering

- Marketing campaigns.
- Paid support tiers.
- Hosted infrastructure.
- Provider-specific integrations.
- Automatic release on every merge.
- Publishing from a developer machine without CI provenance.
