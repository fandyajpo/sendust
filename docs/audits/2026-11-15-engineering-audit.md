# Sendust Engineering Audit — 2026-11-15

**Audit result:** RISK and MEDIUM findings remediated in code; four owner actions remain  
**Audited branch:** `main`  
**Scope:** the delivered P04–P07 code — `crates/sendust-core` (errors, message model, MIME, delivery orchestration, native SMTP engine), `crates/sendust-napi`, `ts/src`, and the CI and documentation that describe them.

This is a code review record. It is not a gate approval, a security certification, or a release approval; the phase gates in `docs/gates/` remain the only completion evidence.

## 1. Findings and resolution

| ID | Finding | Impact | Resolution | Evidence |
| --- | --- | --- | --- | --- |
| RISK-01 | MIME serialization built the full normalized body and a second full base64 payload before comparing anything to the wire cap, so a message under the cap could still allocate several times the cap. | The hard cap did not bound memory; a large message could exhaust the process. | Serialization now preflights the encoded size (allocating nothing), computes the CRLF-normalized length without a buffer, and encodes base64 in 57-byte chunks with the cap checked per output line. | `84cebaa`, `531d104`, `eb25c9b`; tests `oversized_bodies_are_rejected_by_preflight`, `wire_cap_is_inclusive_and_exact` |
| RISK-02 | `close()` during an active send neither aborted nor awaited it; the send could continue on a transport whose phase had already left `OPEN`. | Undefined lifecycle, possible send after close, never shipped — but no automated proof. | The binding now tracks `Active::{Idle, Sending(Option<AbortHandle>)}` plus a `Notify`: `send` spawns the delivery, records its abort handle, aborts when the phase leaves `OPEN`, and maps cancellation to `TRANSPORT_CLOSED` (never `accepted`/`rejected`); `close` aborts and waits within the close budget and reports `TIMEOUT_CLOSE` on expiry. | `88109b2`; tests `aborts a send in flight and rejects it as TRANSPORT_CLOSED`, `closes concurrently without drifting past closed` |
| MEDIUM-01 | A `421` reply was recorded as a per-recipient rejection and the envelope continued. | Wrong recipient statuses and a possible `DATA` on a closing channel. | A `421` now marks the channel closing, keeps the attempted and unattempted recipients `unknown`, and never issues `DATA`. | `d0d4caf`; tests in `crates/sendust-core/tests/smtp_faults.rs` |
| MEDIUM-02 | The implicit-TLS handshake reused the connect budget rather than the time remaining in it. | A slow TCP connect left the handshake with an already-expired budget. | The handshake budget is the remaining connect deadline capped by the remaining total deadline. | `d0d4caf` |
| MEDIUM-03 | The response parser accepted bare `LF` and bare `CR` line endings. | A non-conforming relay could desynchronize the protocol state machine. | Lines must end in `CRLF`; a unit test covers both malformed endings. | `d0d4caf` |
| MEDIUM-04 | The TypeScript facade's lifecycle had no runtime test; coverage stopped at the core. | `close`, `BUSY`, cancellation, and error mapping could regress unnoticed. | Facade tests now cover idempotent close, concurrent close, in-flight send cancellation, refusal after close, and native error mapping, and run in the Native binding CI job against the built addon. | `c615ad7`, `928dc2c`, `e5cccf5`, `9595a88` |
| MEDIUM-07 | Documentation drift: ADR-0006 and ADR-0007 claimed validation by not-started tasks, ADR-0007 described a removed dependency's timeout behaviour, and phase 16, the harness contract, the documentation index, and the phase index reported stale phase states. | Planning documents contradicted the code and each other. | ADRs separate completed validation from pending validation, ADR-0007 describes the native connect deadline, phase 16 and the harness contract name the native engine, and both indexes report the approved P06/P07 state. | This record's commit |

## 2. Verification

The remediated tree was verified on the macOS clone with the pinned toolchain:

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`.
- `cargo test --workspace --all-targets --locked`: 147 core unit tests, 9 harness, 6 client, 4 parser conformance, 6 MIME body fixtures, 20 SMTP fault, 6 session.
- `npm run build:native`, `npm --workspace ./ts run test`: 6 facade tests against the staged addon.
- `npm run lint`, `npm run typecheck`, `node scripts/check-architecture.mjs`, `node scripts/check-docs.mjs`.

GitHub Actions runs the same checks on every push: the Rust workspace, Native binding, TypeScript (Node.js 22/24/26), Bun, Deno, and architecture-and-documentation jobs. The TypeScript, Bun, Deno, and documentation jobs were green on this tree; the Rust workspace and Native binding jobs run the commands above on the same commit.

## 3. Remaining owner actions

| ID | Item | Owner | Why it is not resolved here |
| --- | --- | --- | --- |
| MEDIUM-05 | CI installs with `npm ci --no-audit --no-fund`; there is no advisory dependency-audit stage. | Project owner | Adding a new CI stage changes the governance baseline and must be approved and reflected in the toolchain and dependency policies. |
| MEDIUM-06 | Branch protection for `main` (required checks, no direct pushes) is not configured. | Project owner | A GitHub repository setting; it cannot be changed from this working tree. |
| MEDIUM-08 | `ts/package.json` is `private` with no `files`/optional platform packages and a blocklist `.npmignore`. | P14 distribution | Publishing layout is P14 scope and must not be decided ahead of the distribution gate. |
| MEDIUM-09 | No `LICENSE` file and no `license` field in either manifest. | Project owner | Choosing a license is the owner's decision, not an implementation detail. |

## 4. Method and limitations

- Reviewed the core, binding, and facade source, their tests, the CI workflow, and the documents that describe them.
- The review was static; every claim in section 1 was then verified by running the commands in section 2 and by CI, not by inspection alone.
- Not reviewed: anything P08–P18 owns (attachment file policy, public API contract, reliability hardening, security proof, distribution, release). Findings in those areas are recorded in their phase plans, not here.


## 5. P09 public-contract audit (2026-11-15)

The public API was compared line by line with
[public-api.md](../architecture/public-api.md) and [error-model.md](../architecture/error-model.md)
while P09-001 to P09-003 were implemented. Findings and resolutions:

| Finding | Resolution |
| --- | --- |
| The contract promises unknown option keys are rejected, but the facade forwarded them and the adapter ignored them. | `toNativeOptions` now rejects unknown keys, top-level and inside `auth`, `timeouts`, `limits`, and `attachments`, with `VALIDATION_INVALID_OPTION`. |
| The contract requires a non-empty subject, but the core accepted an empty one. | `OutgoingMessage::validate` rejects an empty or whitespace-only subject with `VALIDATION_INVALID_OPTION`. |
| A missing `from` produced an adapter `TypeError`, not a stable error. | The binding reports `VALIDATION_MISSING_FROM`, and the facade omits an absent field instead of stringifying `undefined`. |
| The contract documents `auth.method`, but only PLAIN was ever attempted (LOGIN as fallback with no way to force it). | Implemented end to end: `with_mechanism`, a `choose_auth` policy that fails an unadvertised forced mechanism, binding mapping, and facade option. |
| The contract promises content-type inference from the filename, but every attachment without an explicit type was `application/octet-stream`. | Added the fixed, documented extension map and applied it in the MIME renderer. |
| The contract exposes error classes, but only a single `SendustError` existed. | One class per category, all extending `SendustError`, selected by `fromNative`. |
| `ehloName` was documented as defaulting to the system hostname or `localhost`, while the implementation always uses `localhost`. | Documentation aligned to the deterministic `localhost` default. |
| Still open: the contract says a close that interrupts an in-flight DATA may leave the accepted subset `unknown`, while `close()` currently aborts the send and rejects it as `TRANSPORT_CLOSED`. | Resolved in P10-001: `DeliveryProgress` records the furthest stage and the RCPT-accepted recipients, so a cancelled send now returns `unknown` for that subset once `data_body` was reached, and `TRANSPORT_CLOSED` before it. |
