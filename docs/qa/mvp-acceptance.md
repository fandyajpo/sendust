# MVP acceptance review (P17-002)

**Status:** matrix complete — every constraint passes; the approve/reject decision
is the project owner's and is recorded in the [PHASE-17 gate record](../gates/phase-17.md).
**Candidate:** `0.1.0-rc.1`, verified in the [RC verification record](rc-verification.md).
**Date:** 2026-11-18.

## 1. Review method

Each row names the authoritative constraint, the artifact that enforces it, and
the evidence checked at the candidate. Requirements are not redefined here; the
[requirements](../product/requirements.md) and [feature matrix](../planning/feature-matrix.md)
remain the source of truth. A row that could not be evidenced would block
publication and be named in section 3.

## 2. Phase-17 acceptance list

| # | Constraint | Result | Evidence |
| --- | --- | --- | --- |
| 1 | Two crates, core free of Node | PASS | `check:architecture`; `cargo metadata` reports exactly `sendust-core` and `sendust-napi`; C-01 |
| 2 | Runtime path `TS -> napi -> core -> native SMTP engine` | PASS | `ts/src` facade over `crates/sendust-napi`, which calls `sendust-core`; the module-layering rule in `check-architecture`; C-02. The native engine replaces the lettre path per ADR-0017 |
| 3 | Sendust-owned SMTP engine, no third-party SMTP dependency | PASS | ADR-0017 accepted; `check-architecture` rejects lettre in production dependencies; P06 approved |
| 4 | No connection pooling | PASS | No pool module or option exists; one connection per send proven by the lifecycle probe (`connections == transports`); C-16 |
| 5 | No automatic retries | PASS | `unknown` is returned, never retried; `ts/test/results.test.ts` and the behaviour harness; C-15, C-16 |
| 6 | Relay-accepted semantics, never delivery | PASS | `results` mapping adds no status; release notes and guides state it; `check-doc-claims` forbids delivery claims; C-15 |
| 7 | `unknown` after DATA | PASS | Behaviour harness "unknown status / recipient / response"; core fault matrix; C-15, C-18 |
| 8 | `BUSY` with no queue and no dial | PASS | Behaviour harness "busy" and "busy connections" (1 connection); lifecycle probe: 50 concurrent sends all `TRANSPORT_BUSY`, 0 extra connections; C-08, C-16 |
| 9 | Explicit, bounded close | PASS | Behaviour harness "close aborts the send", "closed state", "send after close"; close budget 125 s in `config/mod.rs`; C-09 |
| 10 | Finite, lower-only timeouts | PASS | `config/mod.rs` (10 / 30 / 120 / 125 s) and `check-doc-claims` parity; C-10 |
| 11 | Buffers copied before any await | PASS | Adapter copies synchronously; `ts/test` "copies a buffer attachment before the send returns" and "snapshots a buffer before the caller detaches it"; P11 ownership audit; C-11 |
| 12 | Attachments opt-in, regular files only | PASS | `ts/test` "rejects a path attachment without filesystem opt-in" (`CONFIG_FILE_ACCESS_DISABLED`); core symlink/FIFO/device/growth matrix; C-12 |
| 13 | Hard caps 100 / 10 MiB / 20 MiB / 30 MiB, never raisable | PASS | `ts/test/limits.test.ts`; core cap tests; behaviour harness attachment cap; C-13 |
| 14 | Address and Unicode policy | PASS | ASCII local parts, IDNA domains, Unicode display names/subjects/bodies; address and MIME suites; SMTPUTF8 is rejected/documented; C-14 |
| 15 | TLS verification always on, TLS 1.2 floor | PASS | rustls `ring` + `webpki-roots`; every bad-certificate fixture fails before `MAIL`; behaviour harness "tls fails closed"; C-07, C-28 |
| 16 | No public `AbortSignal` or second cancellation path | PASS | `ts/test/types.test.ts` "rejects options outside the frozen surface"; C-17 (stricter than "optional") |
| 17 | Documentation and release notes match behaviour | PASS | `check:docs`, `check:doc-claims`, executable quick start, [release notes](../releases/0.1.0-rc.1.md) reviewed against the guides |

## 3. Constraint ledger

| ID | Constraint | Result | Evidence |
| --- | --- | --- | --- |
| C-01 | Exactly two Rust crates | PASS | `check-architecture`; workspace metadata |
| C-02 | Dependency direction | PASS | Layering rule; no SMTP/MIME policy in `ts/src` or the adapter |
| C-03 | Native SMTP engine | PASS | ADR-0017; no lettre dependency |
| C-04 | One Tokio runtime | PASS | napi-managed runtime; no second runtime created; P11 audit |
| C-05 | Node-API 8 | PASS | `napi8` + `async` features; binding job |
| C-06 | Support claims match CI | PASS | CI matrix; toolchain baseline; [platform matrix](platform-matrix.md) |
| C-07 | TLS verification | PASS | P11 TLS audit; certificate fixtures |
| C-08 | Single active send | PASS | Behaviour harness; lifecycle probe |
| C-09 | Explicit lifecycle | PASS | Behaviour harness; lifecycle suite |
| C-10 | Timeout budgets | PASS | `config/mod.rs`; `check-doc-claims` |
| C-11 | Owned memory | PASS | Copy fixtures; P11 ownership audit |
| C-12 | Attachment filesystem policy | PASS | P11 injection/attachment audit; facade test |
| C-13 | Hard caps | PASS | Facade and core cap tests |
| C-14 | Address and Unicode policy | PASS | Address/MIME suites |
| C-15 | Delivery semantics | PASS | Results and fault suites |
| C-16 | No pooling, retry, or queue | PASS | No such module or option; lifecycle probe |
| C-17 | No public cancellation | PASS | Type-surface test |
| C-18 | Recipient-level outcomes | PASS | SMTP fault matrix (mixed 250/550, one DATA) |
| C-19 | Redaction | PASS | Error redaction test; secret scan |
| C-20 | Stable error taxonomy | PASS | Every stable code mapped to a class and category |
| C-21 | Deterministic test environment | PASS | Loopback-only harness; no external network |
| C-22 | Module formats | PASS | ESM and CommonJS in the packed consumer |
| C-23 | Reproducible, traceable release | PASS | PHASE-16 record; CI bundle; [RC verification](rc-verification.md) |
| C-24 | Evidence-based completion | PASS | Gate records for P03–P16 |
| C-25 | Single source of truth | PASS | Documentation structure check |
| C-26 | Honest support matrix | PASS | Release notes and platform matrix name only tested targets |
| C-27 | No consumer toolchain | PASS | Prebuilt optional platform packages; clean-consumer install |
| C-28 | No plaintext SMTP | PASS | Only implicit TLS and STARTTLS exist |
| C-29 | Injection prevention | PASS | CRLF/NUL/control fixtures in addresses, headers, filenames, EHLO |
| C-30 | Standalone delivery-engine boundary | PASS | No queue, worker, or provider integration; documented non-goals |

## 4. Findings

| Severity | Finding | State |
| --- | --- | --- |
| Low | The phase-17 plan still described the runtime path as `TS -> napi -> core -> lettre` and listed `lettre 0.11.23`, which ADR-0017 superseded | FIXED: the plan now names the native SMTP engine and records that requirement C-03 and ADR-0017 govern |
| Low | The phase-17 text called a public `AbortSignal` "optional"; C-17 prohibits exposing one | Clarified in the plan: the stricter constraint governs, and the API exposes none |

No high- or medium-severity defect, contract deviation, or unsupported claim is
open. The release notes contain no delivery, retry, pooling, SMTPUTF8, URL
attachment, "all platforms", or "all Node versions" claim.

## 5. Decision

The matrix is complete and all 17 acceptance items and 30 constraints pass. The
explicit approve or reject decision belongs to the project owner and is recorded
in the [PHASE-17 gate record](../gates/phase-17.md); a rejection must name the
blocking task and no publication may occur.
