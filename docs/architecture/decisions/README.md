# ADR Index

Architectural decisions for Sendust. One decision per file. Status values: `PROPOSED`, `ACCEPTED`, `SUPERSEDED`, `REJECTED`, `BLOCKED` (requires the named validation task before acceptance). The P02-005 approval matrix is the authoritative list of decisions frozen for the architecture baseline.

| ADR | Title | Status | Owner task |
| --- | --- | --- | --- |
| [0001](0001-two-crates.md) | Two crates: Node-free core plus napi adapter | ACCEPTED (planning) | P02-001 |
| [0002](0002-lettre-over-custom-smtp.md) | Use lettre 0.11.23 instead of a custom SMTP client; low-level API selected | ACCEPTED (planning) | P01-001, P01-005 |
| [0003](0003-tls-rustls-ring-webpki.md) | rustls with ring and webpki-roots, verification mandatory | ACCEPTED (source-reviewed; build gate P03-002) | P01-002 |
| [0004](0004-napi-async-tokio.md) | napi-rs async exports on the napi-managed Tokio 1 runtime | ACCEPTED (design); Node-API 8 selected, runtime tests pending P05/P14 | P01-004, P05-003 |
| [0005](0005-recipient-partial-failure.md) | Recipient partial-failure policy | ACCEPTED (source/API verified; behavior gate P06-003) | P01-003, P06-003 |
| [0006](0006-lifecycle-single-send.md) | One active send, explicit bounded close, no pool | ACCEPTED (planning) | P02-003 |
| [0007](0007-timeout-budgets.md) | Finite timeout budgets and coarse stage attribution | ACCEPTED (design; runtime gate P10-002) | P02-003, P01-003 |
| [0008](0008-owned-memory.md) | Copy JS buffers before async; bounded file reads | ACCEPTED (planning) | P02-004, P05-002 |
| [0009](0009-hard-caps.md) | Hard caps for recipients, attachments, and wire size | ACCEPTED (planning) | P02-004 |
| [0010](0010-mime-composition.md) | Compose MIME in core; use lettre message primitives where correct | ACCEPTED (planning) | P07-001–P07-004 |
| [0011](0011-platform-distribution.md) | Root package plus platform optional packages via napi-rs | ACCEPTED (planning) | P14-002, P14-003 |
| [0012](0012-module-formats.md) | Dual ESM/CJS with a single shared implementation | PROPOSED | P14-002, P14-004 |
| [0013](0013-error-taxonomy.md) | Stable codes, outcomes separate from errors | ACCEPTED (planning) | P02-002, P09-003 |
| [0014](0014-attachment-filesystem-policy.md) | Filesystem attachments opt-in, regular files only, no URLs | ACCEPTED (planning) | P08-001, P08-002 |

## ADR format

Each ADR contains: context, decision, alternatives, consequences, status, evidence, and the task that validates it. An ADR marked `BLOCKED` cannot be relied on for implementation until its validation task produces evidence and the status changes to `ACCEPTED` or `REJECTED` with a replacement.

## Change rule

Before QG-02, `ACCEPTED (planning)` means selected for the draft baseline but not yet frozen. Those planning decisions may be reconciled during P01/P02 source review; the Git history records each amendment, and QG-02 must review the resulting decision text before approval. This is the reconciliation window used by the current architecture audit.

After QG-02 approves the architecture baseline, any material change to an accepted ADR requires a new ADR that supersedes it, a phase-gate review, and updates to every linked contract, task, and test. Silent post-approval edits to accepted ADRs are prohibited.
