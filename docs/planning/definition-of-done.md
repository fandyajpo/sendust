# Definition of Done

Project-wide completion rules. These apply to every task, every phase, and every release. A claim of completion without evidence is a process defect.

## 1. Task-level definition of done

A task may be marked `COMPLETED` only when all of the following are true:

1. **Work performed** — the deliverable described in the task exists in the repository or documentation set.
2. **Prerequisites satisfied** — every prerequisite task is `COMPLETED`, or a gate record explicitly waives it with a risk entry.
3. **Verification** — the task’s evidence criteria are met with linked evidence (test output, report, audit, artifact hash, or review record).
4. **Tests** — if the task changes behavior, tests exist and pass; if it changes documentation only, link and example checks pass.
5. **No regressions** — the phase’s existing checks still pass on the current commit.
6. **Status synchronized** — the phase plan and [task registry](task-registry.md) show the same status.
7. **No placeholders** — no `TODO`, `todo!()`, `unimplemented!()`, “add details later”, or equivalent in the deliverable.
8. **Documentation** — any public behavior changed by the task is reflected in the relevant document, or a linked documentation task is created and scheduled.

## 2. Phase-level definition of done

A phase may be marked `COMPLETED` only when:

1. All tasks in the phase are `COMPLETED`.
2. The phase’s acceptance criteria are met and evidenced.
3. The phase’s completion gate is `APPROVED`.
4. All deliverables are present at the paths named in the plan or an approved equivalent.
5. All mandatory quality gates for the phase pass.
6. Open risks owned by the phase are `CLOSED`, `MITIGATED` with evidence, or `ACCEPTED` with rationale.
7. The next phase’s prerequisites are satisfied.
8. The phase index, task registry, and dependency graph are consistent.

## 3. Evidence rules

Acceptable evidence:

- CI logs tied to a commit.
- Test reports with pass/fail counts and the exact command.
- Benchmark output with environment and repetition count.
- Artifact hashes and manifests.
- Signed gate records.
- Source audits with file/line references.
- Screenshots or transcripts of clean-install runs.
- Published registry metadata for release phases.

Unacceptable evidence:

- Verbal assurance.
- “Works on my machine.”
- A plan or intention.
- An unlinked document that restates the requirement.
- A test that was skipped or marked expected-failure without a tracked risk.
- A benchmark without environment details.

## 4. Definition of done by artifact type

### Code

- Compiles with pinned toolchain.
- Formatting and lints pass.
- Unit tests cover new branches and error paths.
- Integration tests cover the boundary.
- No new `unsafe` without justification.
- No secrets, credentials, or machine-specific paths.
- Public API changes have type tests and docs.

### Tests

- Deterministic; no arbitrary sleeps or external network.
- Failure messages identify the expected and actual behavior.
- Covers at least one negative case per positive case.
- Does not depend on test order or shared mutable state.
- Runs on every declared platform where applicable.

### Documentation

- Every claim matches shipped behavior.
- Examples are executable and tested.
- Links resolve.
- No unsupported platform or Node version is claimed.
- No delivery guarantee, retry, pooling, or SMTPUTF8 implication.
- Terminology matches the [requirements](../product/requirements.md) and [error model](../architecture/error-model.md).

### Release artifacts

- Built from a tagged commit with locked dependencies.
- Reproducible or documented nondeterminism.
- Checksums and provenance present.
- SBOM present.
- Contents match the manifest.
- No secrets or test files.
- Post-publication verification recorded.

## 5. Prohibited completion claims

The following are never sufficient:

- “The code is written.” — must compile and pass tests.
- “Tests pass locally.” — must pass in the declared CI environment.
- “It should work on Windows.” — must be tested or explicitly unclaimed.
- “The error is handled.” — must map to a stable code and be tested.
- “TLS is secure.” — must have no bypass path and negative certificate tests.
- “Performance is fine.” — must have an approved budget and measurement.
- “The package is published.” — must have registry and clean-install verification.
- “The phase is basically done.” — status is binary per gate.

## 6. Status transition rules

| From | To | Allowed when |
| --- | --- | --- |
| NOT STARTED | IN PROGRESS | Prerequisites completed; work begins |
| IN PROGRESS | BLOCKED | A named blocker exists; blocker and owner recorded |
| BLOCKED | IN PROGRESS | Blocker resolved with evidence |
| IN PROGRESS | IN REVIEW | Deliverable and evidence exist; gate review pending |
| IN REVIEW | COMPLETED | Evidence accepted; gate approved |
| IN REVIEW | IN PROGRESS | Review finds gaps; gaps recorded |
| Any | NOT STARTED | Only by an explicit gate decision with a recorded reason (for example, scope reset) |

A task cannot jump from `NOT STARTED` to `COMPLETED`.

## 7. Release-level definition of done

The MVP is done only when:

1. Phases P00–P17 are `COMPLETED` with approved gates.
2. Every REQUIRED feature in the [feature matrix](feature-matrix.md) has implementation and test coverage.
3. Every quality gate in [quality-gates.md](quality-gates.md) passes on the release commit.
4. The published package installs and runs on every declared target and Node version.
5. Documentation, support matrix, and release notes are internally consistent.
6. Post-publication verification succeeds.
7. Rollback readiness is recorded.
8. No post-MVP feature is present in the MVP release.

## 8. Change control

- Requirements change: update [requirements.md](../product/requirements.md), the feature matrix, and affected phase tasks; review at the next gate.
- Architecture change: new or superseding ADR; update system design, data flow, and error model; review at P02 or the relevant gate.
- Task addition: add to the phase plan with the next stable ID; update the registry; verify dependencies.
- Task removal: requires a gate record and a check that no requirement or feature loses coverage.
- Task renumbering after work begins: prohibited.

## 9. Audit checklist

Before declaring a phase complete, answer all of these with “yes” or a linked exception:

- Are all task IDs present and unique?
- Does every task have a size, status, and prerequisite list?
- Does every prerequisite exist and precede the task?
- Does every acceptance criterion have evidence?
- Do all tests named in the plan exist and pass?
- Are risks updated?
- Are links valid?
- Is the registry synchronized?
- Is the next phase unblocked?
