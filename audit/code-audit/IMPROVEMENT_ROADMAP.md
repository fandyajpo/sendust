# Improvement Roadmap — Sendust 1.2.1

Prioritised from the audit's findings. **Nothing here was implemented**; this is a plan, and
every item names the evidence it answers. Ordering is by risk × effort, not by area.

Guiding rule taken from the audit's own conclusions: *delete or wire a contract; never add a
third copy of it.*

---

## 1. Immediate — before the next release

### 1.1 Fix H-01 — the 512-byte `AUTH` cap

**Why now:** it makes XOAUTH2 unusable against the providers most likely to require it, and it
fails with a code that accuses the caller of injection.

**What:** stop applying `MAX_COMMAND_BYTES` to the `Auth` and `AuthResponse` variants. RFC 4954
§4 permits an initial response beyond the 512-octet command line, and servers must accept at
least 1,228 octets; the 12,288-byte validator at `proto/command.rs:74, 85` already encodes the
intended bound. Map an over-long credential to `VALIDATION_INVALID_OPTION`, never to
`VALIDATION_HEADER_INJECTION`.

**Proof required:** a test that builds a `Command::Auth` with a 1,500-byte bearer token and
asserts the line serialises; a session test whose fake relay advertises `AUTH XOAUTH2` and
accepts a long token. Without those two, the fix will regress.

**Effort:** hours. **Risk of the fix:** low — one predicate, scoped to two variants.

### 1.2 Fix H-02 — a failed delivery must not brick the transport

**Why now:** one internal panic turns a healthy transport into one that answers `BUSY`
forever, with the wrong cause.

**What:** the slot must be refilled on *every* exit from the spawned task. Two changes:

1. move the transport back into the slot inside the task's own final step, or return ownership
   through the `JoinHandle`'s `Err` path by keeping the transport outside the task (for
   example in an `Arc<Mutex<Option<SmtpTransport>>>` the task borrows and returns);
2. when the task ended abnormally, report `INTERNAL` for that send and mark the transport
   unusable *explicitly* (a `failed` phase) instead of leaving `slot == None`, which is
   indistinguishable from "someone else is sending".

**Proof required:** a test that forces the task to panic (a `cfg(test)` injection point, or a
deliberate panic behind a feature flag) and asserts the next `send` returns `INTERNAL`/a
documented failed state — never `TRANSPORT_BUSY` forever.

**Effort:** half a day. **Risk:** medium — this is the ownership core of the binding; land it
with the test.

### 1.3 Correct the documentation the code contradicts

Four claims are currently false; each is a one-line correction and each is listed in
`CODE_QUALITY_REVIEW.md` §5. Fixing them is cheaper than the trust they cost when a user
discovers them: `validate_in_order` (§M-06), `PROTOCOL_REJECTED` (M-11), the two unproducible
error details (M-10), and the host-whitespace contract (M-07). Do the doc correction *and*
the code decision (wire or delete) in the same change.

**Effort:** hours.

---

## 2. Short-term — next one or two minor releases

### 2.1 Close the reliability gaps in the wrapper

| Item | Finding | Change |
| --- | --- | --- |
| Release a pooled transport exactly once | M-02 | Move `#release` into `finally` against a nulled local; add a test where `toSendResult` throws |
| Reset cancellation progress before composition | M-01 | Call `progress.reset()` at the start of `Transport::send`, not inside `SmtpTransport::run`, so the abort window cannot inherit the previous message's stage and recipients |
| Check the `RSET` reply | M-03 | Treat a non-2xx `RSET` as a reason to drop the session, per ADR-0022's own wording ("only when the relay answers the `RSET` that closes the transaction") |
| Surface the fatal state | M-09 | Add `failed` to `TransportState`, or a `fatal` flag on `TransportStats`, and document in-process recovery |
| Make `sessionIdleMs: 0` unambiguous | L-12 | Either reject 0 or make both sides agree; today the engine never retires and the core always expires |

**Effort:** one to two days total. All five are covered by existing test patterns
(`ts/test/transport.test.ts` has a fake relay; `tests/smtp_reuse.rs` has session machinery).

### 2.2 Enforce `SIZE`

**Finding:** M-04.

`Capabilities::size()` is parsed and never read. Compare the composed wire length against it
before `MAIL FROM`, and emit `SIZE=<n>` on `MAIL FROM` when the relay advertises the extension
(RFC 1870). The wire length is already authoritative at that point — `DeliveryRequest.wire` is
exactly what will be sent, signature included.

Failing locally with a clear limit error beats being disconnected mid-`DATA`, which is the
failure this prevents — and a disconnect mid-body would be reported as `unknown`, forcing a
human to decide whether to resend.

**Effort:** half a day including tests. **Note:** `MAIL FROM` parameter support is a small
addition to `Command::MailFrom`; keep the injection rules intact.

### 2.3 Restore parity between runtime testing and the runtime claim

Run the `binding` job as a Node matrix (22.23.5, 24, 26) instead of a single version
(`ci.yml:133-190`), and include `test/**/*.ts` in `tsconfig.json` so the whole suite is
type-checked (M-12). Both are configuration changes; the second may surface latent type
errors in `transport.test.ts` (997 lines) and should be done early so they are fixed while the
context is fresh.

**Effort:** hours, plus whatever the type errors cost.

### 2.4 Harden the two documented model risks

* **M-08** — documentation, not code: state plainly that `attachment.path` must never be
  derived from untrusted input, and that enabling file access means "read any regular file
  this process can read".
* **I-10** — document Deno's `--allow-ffi` / `--allow-read` requirement in the compatibility
  section, since it narrows the sandbox a Deno user chose.
* **I-08** — state that the DKIM key and any bearer token live in JavaScript strings for the
  process lifetime, so callers can reason about key rotation.

**Effort:** hours. **Value:** high — these are the three places where a user's mental model
is likely wrong in a way that matters.

### 2.5 Add dependency advisory scanning

`cargo-audit` and `npm audit --omit=dev` as a non-blocking CI job. The dependency count is
tiny and every version is pinned, so this will almost always be green — which is exactly why
it is cheap to add and worth having when it is not.

---

## 3. Long-term — architectural

### 3.1 Resolve the duplicated lifecycle (M-05)

Two honest options; pick one and record it in an ADR:

1. **Delete `core/src/lifecycle`** and state that the binding owns the state machine. Smallest
   change, and it makes the existing gate evidence honest by removing tests that no longer
   describe the product.
2. **Move the phase machine into the core** and have the binding drive it. Restores ADR-0016's
   intent, gives the shipped lifecycle direct unit tests, and would let a second runtime
   binding (if one ever exists) reuse it.

Option 2 is more work but it is the one that matches the architecture the project advertises;
option 1 is the one that matches the code. Either way, do not keep both — a third copy is the
only outcome worse than today.

### 3.2 Delete or wire the validation ordering (M-06)

`validate_in_order` has no caller. If determinism of error precedence is a real requirement
(as the docs claim), the cheapest honest implementation is a single function per stage called
in order, with a test per adjacent pair — which `validation/mod.rs` already contains for the
abstraction nobody calls. Wiring it would also let `mime::compose` stop re-validating entirely
(L-14).

### 3.3 Remove the unproducible surface (M-10, M-11)

Either populate `details.recipient` and `details.smtpEnhancedCode` at the points where the
information exists — the `RCPT` loop has the address, `Reply::enhanced()` has the RFC 3463
code — or delete them and the `EnhancedStatus` parser with them. `PROTOCOL_REJECTED` should
be deleted: definitive refusals are outcomes by design, and a protocol error code that no path
can produce only invites callers to write dead branches.

### 3.4 Streaming composition (M-13)

Today the wire exists at least twice in memory before `DATA` (composed, then signed), and the
body is base64-encoded into that buffer. With a hard 30 MiB wire cap and a 20 MiB attachment
cap this is bounded, but a 32-way concurrent burst multiplies it, and it is the reason large
attachments are a memory event rather than an I/O event.

A staged design — write the body to the socket through a signing layer that hashes as it
streams, with the `DKIM-Signature` header prepended — would cut peak memory to the header set
plus a fixed buffer. It is a significant change: the relaxed body canonicalisation and the
`bh=` hash must be computed on the same bytes that are dot-stuffed and sent, and the wire cap
must become a running counter rather than a post-hoc check. Worth an ADR before code.

### 3.5 Protocol coverage

In rough order of value:

1. **`HELO` fallback** when `EHLO` is refused — cheap, and RFC 5321 requires clients to try it.
   Today an EHLO refusal is `PROTOCOL_UNEXPECTED_RESPONSE` with no retry.
2. **`BODY=8BITMIME`** is unnecessary while bodies are base64; skip it unless a
   `Content-Transfer-Encoding: 8bit` option is ever added.
3. **`SMTPUTF8`** — currently rejected by design for the local part (`address/mod.rs:201-203`)
   and declared post-MVP. Keep it that way until there is a requirement.
4. **`PIPELINING`** would remove round trips from the `RCPT` loop, which matters at 100
   recipients. It also multiplies the failure modes in the transaction state machine, so only
   attempt it with the fault-injection harness extended first.

### 3.6 Tooling that would have caught this audit's findings

| Finding | Test that would catch it |
| --- | --- |
| H-01 | An `AUTH` line at 1,500 bytes in `smtp_session.rs` |
| H-02 | Panic injection at the binding boundary |
| M-02 | `toSendResult` throwing in `engine.test.ts`, asserting one pool slot |
| M-05 | An architecture check that every `pub use` has a non-test consumer |
| M-10/M-11 | A test that every documented code is producible by some path, or that every code in the docs table is reachable |
| M-04 | A fake relay advertising `SIZE 1024` and a 2 KiB message |

The last two are the highest-leverage: they turn "documentation drift" from a review finding
into a CI failure.

---

## 4. Explicitly not recommended

* **Rewriting the SMTP client on `lettre`.** ADR-0002 evaluated and rejected it; the native
  client is now the reason the I/O model, deadlines and outcome mapping fit together at all.
* **Adding a persistent queue or scheduler.** Correctly excluded by the product boundary; the
  in-process engine plus caller-owned durability is the right split.
* **Relaxing TLS for local relays.** The fixture-based test path already covers local testing
  through `tlsRootCertificate`; a "no verification" switch would be the single most dangerous
  addition possible to this codebase.
* **Chasing Nodemailer's feature surface.** The competitive position (documented in
  `docs/research/competitive-landscape.md`) is narrow-and-correct, not broad. Every protocol
  feature added is a new state machine to fault-inject.

---

## 5. Suggested sequencing

| Order | Item | Gate |
| --- | --- | --- |
| 1 | H-01 fix + long-token test | patch release |
| 2 | H-02 fix + panic test | patch release |
| 3 | Doc corrections (M-06, M-10, M-11, M-07) | same release as 3 |
| 4 | Wrapper reliability set (§2.1) | next minor |
| 5 | `SIZE` enforcement (§2.2) | next minor |
| 6 | CI parity (§2.3): Node matrix + full typecheck | next minor |
| 7 | Model-risk documentation (§2.4) | next minor |
| 8 | Delete-or-wire the four dead contracts (§3.1–3.3) | next major, or a dedicated cleanup minor |
| 9 | Streaming composition (§3.4) | after an ADR; major |
| 10 | `HELO` fallback, `PIPELINING` (§3.5) | opportunistically |

Steps 1–3 are small, testable and directly reduce user-visible risk; they should not wait for
a scheduled release train. Steps 8–9 are the difference between a good 1.x library and one
whose documentation can be trusted without reading the code.
