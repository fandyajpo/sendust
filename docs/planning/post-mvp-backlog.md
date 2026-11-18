# Post-MVP backlog and next-cycle handoff (P18-005)

**Status:** prepared — the PHASE-18 decision is the project owner's.
**Date:** 2026-11-18.
**Baseline:** `@sendust/ts@0.1.0-rc.1`, released 2026-11-18
([PHASE-17 record](../gates/phase-17.md)).

This is the prioritized backlog the post-MVP phase hands off. It contains no
implementation: every item names its rationale, its dependencies, the ADR it
would need, the evidence it would need, and the entry criteria for starting it.

## 1. No must-fix defects

The [triage log](../qa/post-mvp-intake.md) classifies every item open at release.
None is a defect against the published artifact: the release is hash-verified from
the registry and its behaviour contract passes on Node.js, Bun, and Deno. The
published MVP and its hashes stay unchanged.

## 2. Priority 1 — process and documentation

| Item | Rationale | Dependencies | ADR | Evidence | Entry criteria |
| --- | --- | --- | --- | --- | --- |
| Create the annotated tag `v0.1.0-rc.1` on `a8a71e7` | The checklist requires the tag and it is not moved afterwards | None | None | Tag visible on GitHub | Owner action; no code |
| Move off bypass-2FA tokens before January 2027 | npm deprecates bypass-2FA direct publishing | A trusted publisher or a stage-only token with maintainer promotion | None | A published release using the new path | Owner configures npm; then the next release uses it |

## 3. Priority 2 — capability candidates

Every item here is **deferred**: it needs its own ADR and release gate, and none
starts in P18.

| Item | Rationale | Dependencies | ADR | Evidence | Entry criteria |
| --- | --- | --- | --- | --- | --- |
| SMTPUTF8 | Non-ASCII envelope local parts | Relay capability detection, validation, fixtures | New ADR superseding the ASCII-local-part rule | Relay matrix, encoding fixtures, downgrade policy | A relay that advertises and honours the extension, plus fixtures |
| Public `AbortSignal` | Familiar cancellation for callers | Bounded close mapping, `unknown` after `DATA`, race tests | New ADR superseding C-17 | Outcome table, race tests, docs | Proof that no abort reports a false `accepted` or `rejected` |
| Connection reuse within one transport | One fewer handshake per message | Session-lifetime evidence, close/drain semantics, leak probes | New ADR amending ADR-0006 | The evidence list in the [pooling research](../research/connection-pooling.md) | Relay idle-lifetime data and reuse tests on every target |
| Platform expansion (Windows arm64, Linux musl) | Wider install base | Native runner, packaging metadata, loader entry | ADR-0020 amendment | Behaviour suite on the target, matrix update | A runner that builds and loads the target natively |

## 4. Priority 3 — deferred engineering

| Item | Rationale | Dependencies | ADR | Evidence | Entry criteria |
| --- | --- | --- | --- | --- | --- |
| Strip development metadata from the publish manifest | Cleaner package | A new version and facade hash | None | Tarball diff and `check-pack` | Scheduled with the next version |
| CycloneDX or SPDX SBOM | Machine-readable inventory | Tooling choice | None | SBOM plus inventory diff | Decided with the next release line |
| macOS signing and notarization | Installer trust | Apple account and CI signing | None | Signed artifact and a load test | Owner decision |

## 5. Rejected, with reasons

| Item | Why it stays out |
| --- | --- |
| Delivery confirmation, open/bounce tracking | Not observable to a client; contradicts the relay-accepted contract |
| Automatic retry | Duplicates mail after `unknown`; retry belongs to the caller with application-level idempotency |
| Internal queue, worker, scheduler, persistence, queue adapters | The product is a library; orchestration belongs to the consuming application |
| URL or remote attachments | Adds a network and SSRF surface to an opt-in file reader |
| TLS bypass or plaintext mode | Verification is always on |
| SMTP server mode | Sendust is a client, not a relay |
| A pool with an internal queue | Contradicts one active send and `BUSY` |

## 6. Handoff

- The published MVP stays `0.1.0-rc.1` with its hashes and its approved gate
  records; nothing in this phase changed code, tests, packages, or artifacts.
- Any future phase starts from one backlog row, with its ADR drafted first and its
  evidence criteria agreed before implementation.
- The compatibility rules in the [compatibility policy](../engineering/compatibility-policy.md)
  decide whether a proposal is a minor or a major release.

## 7. Evidence for the phase gate

- The backlog is prioritized and every row has rationale, dependencies, required
  ADR, evidence, and entry criteria (sections 2 to 4).
- Rejected items include reasons (section 5).
- The MVP baseline and published hashes are unchanged (sections 1 and 6).
- No post-MVP implementation started in this phase.
