# Post-MVP candidate matrix (P18-002)

**Status:** complete — research only. Nothing here is implemented, approved, or
claimed.
**Date:** 2026-11-18.
**Baseline:** `@sendust/ts@0.1.0-rc.1`, the released MVP
([PHASE-17 record](../gates/phase-17.md)).

This record evaluates the candidate capabilities the MVP deliberately excludes.
Each row states the benefit, the risk, the compatibility impact on the released
contract, the ADR that would be required, the evidence that would be needed, and
the decision taken in this phase. A decision of "defer" means the candidate stays
out of the product; it does not authorize work.

## 1. Candidate matrix

| Candidate | Benefit | Risk | Compatibility | Required ADR | Evidence needed | Decision |
| --- | --- | --- | --- | --- | --- | --- |
| SMTPUTF8 (RFC 6531) | Non-ASCII envelope local parts and UTF-8 addresses | Relay support varies; encoding mistakes reach the wrong mailbox; bounces and relays that strip the extension cannot be detected from acceptance | Additive for current callers, but changes envelope semantics and the validation contract | New ADR superseding the ASCII-local-part rule | Relay capability matrix, encoding fixtures, per-relay behaviour, downgrade policy | **Defer**; remains post-MVP |
| Public `AbortSignal` | Familiar cancellation for callers already using `AbortController` | Cancellation mid-`DATA` cannot be safe: the message may have been accepted, so the outcome becomes `unknown`; races against `close` | Constraint C-17 prohibits it today; adding it changes the lifecycle contract | New ADR mapping the signal to bounded close, never required shutdown | Definition of the observable outcome on abort, race tests against close and BUSY, docs | **Defer**; the supported mechanism stays explicit `close()` |
| Connection pooling | Lower per-message connection cost in high-volume senders | Breaks one-active-send and `BUSY`; stale sessions; relay rate limits; close must drain | Would change ADR-0006 and the `BUSY` contract | ADR plus a compatibility analysis | Session health, concurrency, resource bounds, close/drain interaction | **Defer to P18-003**, which studies it as separate future research |
| Additional platforms (Windows arm64, Linux musl, other runtimes/platforms) | Wider install base | An untested target becomes an unsupported claim; a new runner, signing, and packaging path per target | Additive only when `os`/`cpu`/`libc` metadata and the loader map agree | ADR-0020 amendment | A native CI runner, the per-target behaviour suite, distribution metadata, and platform-matrix evidence | **Defer** until demand and runner availability are real |
| Additional Node.js versions | Reach older deployments | The pinned engines line is a support claim; untested versions can break the addon | `engines.node` and the support matrix would widen | None (policy change) | Full suite and behaviour run on each version | **Keep** the tested line only |
| Richer recipient reporting (per-recipient delivery, DSN handling) | More detail for callers | Implies delivery confirmation, which the product explicitly does not claim | Would contradict relay-accepted semantics | A product decision, not only an ADR | Not applicable until the product boundary changes | **Reject** for the current product |

## 2. SMTPUTF8

Today envelope local parts must be ASCII and domains are IDNA-converted; display
names, subjects, and bodies are already Unicode via MIME. SMTPUTF8 would extend
the envelope itself.

What would have to be true before an ADR could even be drafted:

- the relay advertises `SMTPUTF8` in `EHLO` and the client detects it per session;
- a message with a non-ASCII local part is either sent with the extension or
  rejected before `MAIL FROM`, never silently downgraded;
- fixtures cover a relay that advertises the extension but corrupts the address;
- documentation states that acceptance still means relay acceptance only.

The native engine already parses capabilities, so the detection point exists; the
open questions are policy, relay evidence, and validation, not plumbing.

## 3. `AbortSignal`

The supported cancellation is `close()`, which is explicit, idempotent, bounded by
125 s, and never GC-dependent. A signal could only be a convenience wrapper with a
strict mapping:

- aborting before `DATA` finishes behaves like `close` and the send rejects with
  `TRANSPORT_CLOSED`;
- aborting after `DATA` has been transmitted cannot be reported as cancelled: the
  outcome must be `unknown`, exactly as a lost final reply is today;
- the transport still reaches `closed` through the same bounded path.

C-17 keeps this out of the MVP. A future ADR must prove that no abort path can
report a false `rejected` or `accepted`.

## 4. Platform expansion

Every claimed target is built and loaded natively in CI, and the platform matrix
lists only tested OS/CPU/ABI combinations. A new target therefore needs its own
runner, its own behaviour run, package metadata (`os`, `cpu`, `libc`), a loader
entry, and a matrix update. Windows arm64 and Linux musl are the plausible next
targets; neither has a proven runner here, so neither is added.

## 5. Rejected candidates

These are not deferred research; they are outside the product:

- **Delivery confirmation, open and bounce tracking.** Sendust reports what the
  relay did. Mailbox delivery is not observable to it.
- **Automatic retry.** After `unknown`, retrying can duplicate mail. Retry is a
  caller decision that needs application-level idempotency.
- **Queue, worker, scheduler, persistence, and queue adapters.** The boundary in
  [README](../../README.md) and the [MVP scope](../product/mvp-scope.md) places
  these in the consuming application.
- **URL or remote attachments.** Attachment reads are opt-in, regular-file only,
  and bounded; fetching remote content adds a network and SSRF surface.
- **TLS verification bypass or a plaintext mode.** Verification is always on.
- **SMTP server mode.** Sendust is a client library, not a relay.

## 6. Evidence for the phase gate

- The matrix lists benefit, risk, compatibility, required ADR, and evidence
  needed for every candidate (section 1).
- SMTPUTF8 remains explicitly post-MVP until an ADR proves relay support and
  encoding behaviour (sections 1 and 2).
- `AbortSignal` maps to bounded close semantics and never to a required shutdown
  (section 3).
- Platform expansion requires lifecycle and distribution evidence before any
  claim (section 4).
- No code, test, package, or published artifact changed: this record is research
  only.
