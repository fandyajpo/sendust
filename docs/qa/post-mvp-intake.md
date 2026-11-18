# Post-MVP intake and defect triage (P18-001)

**Status:** complete — intake channels, templates, classification rules, and the
triage log for every open item.
**Date:** 2026-11-18.
**MVP baseline:** `@sendust/ts@0.1.0-rc.1`, published 2026-11-18
([PHASE-17 record](../gates/phase-17.md)).

This record is the P18-001 deliverable. It defines how post-MVP feedback arrives,
what an item must contain before it can be triaged, and how each class is
handled. It changes no MVP semantics and no published artifact.

## 1. Channels

| Channel | Use | Path |
| --- | --- | --- |
| GitHub issue, defect template | A behaviour that contradicts the published contract | Repository issue with the fields in section 2 |
| GitHub issue, feature template | A capability the MVP deliberately excludes | Repository issue; triaged by P18-002 or P18-003 |
| GitHub Security Advisory | Anything that could weaken TLS, redaction, attachment reads, or the FFI boundary | Private advisory, never a public issue |
| Documentation issue | A guide, release note, or error message that misstates behaviour | Repository issue; classified as a documentation gap |

There is no support inbox, SLA, or private email channel. A reporter who cannot
use GitHub is asked to open an issue with redacted content only.

## 2. Intake template

Every item carries these fields; an item without them is returned, not guessed at.

| Field | Why it is required |
| --- | --- |
| Version | `version()` of the installed package, e.g. `0.1.0-rc.1` |
| Platform | OS, CPU architecture, libc where relevant, and how it was installed (`npm`, Bun, Deno) |
| Runtime and version | Node.js `22.23.5`, Bun `1.4.4`, Deno `2.9.8`, or the exact other version |
| Relay behaviour | Implicit TLS or STARTTLS, the relay's final reply code, and whether the connection survived |
| Result status | `accepted`, `rejected`, `unknown`, or the error `code` |
| Lifecycle state | Whether a send was active, closing, or closed at the time |
| Attachments | Whether `allowFileAccess` was enabled and which caps applied |
| Redacted diagnostics | Codes, stages, and counts only — never credentials, bodies, attachment bytes, recipients, or absolute paths |

## 3. Classification

| Class | Definition | Handling |
| --- | --- | --- |
| Defect | Published behaviour contradicts the guides, release notes, or an approved gate record | Must-fix candidate; reproduce on the published artifact, then a new version and a release gate |
| Documentation gap | Code is correct, the documentation is wrong or missing | Fix the document; no version bump needed |
| Feature request | A capability the MVP excludes on purpose: SMTPUTF8, `AbortSignal`, pooling, retry, URL attachments, delivery confirmation, new platforms | Feeds P18-002 or P18-003; never implemented in this phase |
| Unsupported use | The report asks Sendust to be a queue, scheduler, campaign tool, SMTP server, or delivery tracker, or relies on an unclaimed platform | Closed with a pointer to the boundary in [README](../../README.md) and the [MVP scope](../product/mvp-scope.md) |

A report is never resolved by quietly widening a guarantee: relay acceptance is
not delivery, `unknown` is not success, and no retry is added to cover a defect.

## 4. Triage log

Every open item at the moment the MVP shipped, classified and dispositioned.

| Item | Source | Class | Disposition |
| --- | --- | --- | --- |
| npm assigned `latest` to the pre-release alongside `next`, and the registry refuses to remove it (400) | PHASE-17 finding | Documentation gap | Closed: release notes and the publication checklist name `@sendust/ts@next` and the exact version; the first stable release moves `latest` |
| The Windows DLL is not byte-reproducible across CI runs | PHASE-17 finding | Not a defect — documented nondeterminism | Closed: publication identifies Windows by the approved build inputs and the bundle's checksums |
| The publish manifest keeps the repository's `scripts` and `devDependencies` | PHASE-16 finding | Feature request | Deferred to a later version: stripping them changes the facade hash, so it needs a new candidate |
| No CycloneDX/SPDX SBOM; the dependency inventory stands in for it | PHASE-16 deferral | Feature request | Deferred; revisit with the next release line |
| macOS binaries are neither signed nor notarized | Release notes limitation | Feature request | Deferred; not part of the MVP claim |
| The annotated tag `v0.1.0-rc.1` is not created yet | PHASE-17 closure | Process item, owner action | Closed 2026-11-18: the tag was created on `a8a71e7` and pushed |
| The npm bypass-2FA token is deprecated for direct publishing in January 2027 | Publication checklist | Operational item | Open: migrate to trusted publishing or stage-only with maintainer promotion before then |
| SMTPUTF8 is excluded; envelope local parts must be ASCII | MVP scope | Feature request | P18-002 candidate |
| No public `AbortSignal` | Constraint C-17 | Feature request | P18-002 candidate, mapped to bounded close, never required shutdown |
| Connection pooling is excluded; one active send per transport | ADR-0006, MVP scope | Feature request | P18-003 research only |
| Queueing, workers, scheduling, and queue adapters are outside the product | README boundaries | Unsupported use | Closed by design; no adapter contract will be produced |
| Windows arm64, musl, and other targets are unclaimed | Platform matrix | Feature request | P18-002 candidate; requires lifecycle and distribution evidence first |
| Delivery confirmation, open/bounce tracking | README boundaries | Unsupported use | Closed: not planned without a separate product decision |

No item in this log is a defect against the published artifact.

## 5. Evidence for the phase gate

- Templates capture version, platform, runtime, relay behaviour, result status,
  and redacted diagnostics (section 2).
- Every item is classified as defect, documentation gap, feature request, or
  unsupported use (section 4).
- Security reports have a private path (section 1).
- No item was accepted by changing MVP semantics; the baseline stays
  `0.1.0-rc.1` with its published hashes.
