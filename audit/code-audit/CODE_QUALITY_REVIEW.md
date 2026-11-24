# Code Quality Review — Sendust 1.2.1

Rust quality, TypeScript quality, native-binding quality, maintainability, and the full LOW
finding register. Static review only; nothing was executed.

---

## 1. Rust quality

### 1.1 Safety

* **No `unsafe` in production code.** Neither `sendust-core/src` (25 files) nor
  `sendust-napi/src` (2 files) contains an `unsafe` block — verified by search, not by
  sampling. The only two in the workspace are test-only: a `libc::mkfifo` call that builds a
  FIFO fixture (`attachments/mod.rs:730`) and a counting global allocator used to measure
  allocation (`tests/perf_attachments.rs:32-41`, an integration test rather than crate code).
  A safety-critical SMTP client written with zero production `unsafe` is a meaningful claim
  and it holds.
* **No panicking `unwrap`/`expect` on a fallible operation in production.** Every fallible
  path uses `?`, `ok_or_else`, `map_err` or a total fallback (`unwrap_or_default`,
  `unwrap_or(0)`, `saturating_*`). Mutex poisoning is handled explicitly rather than by
  `unwrap`: `unwrap_or_else(PoisonError::into_inner)` in `results/mod.rs:115` and
  `lifecycle/mod.rs:141`. `expect()` appears only inside `#[cfg(test)]` modules.
* **Overflow is designed away.** `u64::try_from(...).unwrap_or(u64::MAX)`,
  `saturating_add`/`saturating_duration_since` in the deadline and cap arithmetic
  (`config/mod.rs:221-257`, `mime/mod.rs:441-457`).

### 1.2 Ownership and allocation

* Buffers are moved, not cloned, along the hot path: `DeliveryRequest` takes ownership of
  both `message` and `wire` (`ports.rs:16-19`), and the port is handed `&mut self` so the
  binding can keep the transport (`ports.rs:33-37`).
* The one deliberate clone is the RSA key: `SigningKey::<Sha256>::new((*self.key).clone())`
  per signature (`dkim/mod.rs:202`). `RsaPrivateKey` is not `Copy`, so this allocates two
  bignums per send. For a per-message DKIM sign on a ≤30 MiB wire it is negligible; it is
  nevertheless the only per-send allocation that could be avoided by holding a prebuilt
  `SigningKey` in the signer. Not raised as a finding — the signer is `Clone` by design.
* Base64 is streamed in 57-byte chunks straight into the output buffer, so a complete encoded
  string is never materialised (`mime/mod.rs:487-500`). The comment explains the choice
  (57 → exactly one 76-char line). This is the kind of detail that usually gets written wrong.
* `std::mem::take(&mut message.attachments)` avoids a clone while loading files
  (`delivery/mod.rs:88`).

### 1.3 Error propagation

The taxonomy is the strongest non-security asset in the codebase:

* Construction is restricted: `Error::new(code)` plus a whitelisted `ErrorDetails` builder
  (`errors/mod.rs:175-305`). There is no path from arbitrary text to an error message.
* `Error` is `Clone + PartialEq + Eq`, so tests assert on codes rather than strings.
* Codes are append-only by policy and checked against a documentation table by a unit test
  (`errors/mod.rs:321-374`), including uniqueness.
* Enrichment never rewrites a code — `annotate` rebuilds details and keeps
  `error.code()` (`smtp/mod.rs:745-776`).

The one weakness is that the taxonomy declares three things nothing produces (M-10, M-11 in
§7): a contract is only a contract if it can be observed.

### 1.4 Unsafe deserialisation / untrusted parsing

The reply parser is hand-written and total (`proto/response.rs:163-241`): it requires CRLF,
rejects bare LF and bare CR, bounds each line at 1000 bytes and a whole reply at 64 KiB,
refuses mismatched multiline codes, and rejects a continuation with no final line. There is
no `serde`, no format string constructed from input, and no recursion. This is the correct
way to parse a hostile protocol.

### 1.5 Duplication

| Site | Nature | Assessment |
| --- | --- | --- |
| `relaxed_header` / `relaxed_line` | `dkim/mod.rs:289-359` reimplements header and body relaxation instead of reusing `mime/headers.rs` | **Justified**: DKIM canonicalisation is a different algorithm from RFC 5322 folding, and sharing them would couple two independently specified transforms |
| `normalize_body` ↔ `normalized_len` | `mime/mod.rs:233-252` and `:460-476` | **Acceptable**: one allocates, one measures without allocating; a shared iterator would complicate the read path |
| `fold` ↔ `fold_header` | `mime/headers.rs:104-133` and `dkim/mod.rs:366-398` | **Justified**: header folding breaks at any space; the DKIM folder must break only between tags, or the signature changes |
| Lifecycle state machine | `lifecycle/mod.rs` vs `napi/lib.rs:142-157, 308-453` | **Not justified** — M-05 |
| Validation ordering | `validation/mod.rs` vs inline in `message/mod.rs` | **Not justified** — M-06 |
| "restore slot + reset active + notify" | `napi/lib.rs:325-336, 341-344, 350-354` | Four near-identical restore blocks in one function; a `ScopeGuard`-style helper would be safer against a future edit that restores two of three |

### 1.6 Module boundaries

Clean and consistently held. `lib.rs` re-exports a curated surface (`lib.rs:23-41`); the
internal seams needed by integration tests live behind `__internal` with a "not a supported
API" doc comment (`lib.rs:43-52`) — a rare and correct compromise. `transports/smtp/{client,
proto,session}` separate transport, codec and transaction, and `proto` has no I/O.

### 1.7 Comments

Comments explain *why*, and several are load-bearing. Examples worth citing:

* `connection.rs:122-124` — why the TLS handshake shares the connect deadline instead of
  restarting it.
* `dkim/mod.rs:362-365` — why folding at a `;` is safe (relaxed canonicalisation restores
  that space).
* `results/mod.rs:107-110` — why progress must be reset between sends (the seed of M-01:
  the reset exists but sits too late).
* `leaks.rs:72-82` — why the probe serialises a runtime to avoid a false "leak" on CI.

This is the layer where a 1.x library usually fails; here it is a strength.

---

## 2. TypeScript quality

### 2.1 Type safety

`tsconfig.json` enables `strict`, `exactOptionalPropertyTypes`, `noUncheckedIndexedAccess`,
`noImplicitOverride`, `useUnknownInCatchVariables`, `verbatimModuleSyntax` and
`isolatedModules` (`ts/tsconfig.json:7-13`). `exactOptionalPropertyTypes` in particular is
rare and matters here: `{ pass?: string }` cannot be assigned `undefined`, which is what makes
the "exactly one secret" rule in `nativeAuth` (`transport.ts:132-156`) expressible at all.

* **No `any` in the public surface.** Untyped native data is consistently `Record<string,
  unknown>` or a locally declared structural interface. The only assertions are
  `as unknown as` casts confined to runtime detection (`engine.ts:83`, `loader.ts:147`) and one
  narrowed record cast (`transport.ts:173`).
* **The public surface is frozen deliberately.** `types.test.ts` asserts the exact key set of
  every option and message type (`types.test.ts:59-97`), so an accidental addition fails CI.
* **Unknown keys are rejected at runtime** (`assertKnownKeys`, `transport.ts:98-104`), so a
  typo in `delivery` fails loudly instead of being ignored — a class of bug that silently
  costs users their retry settings.

### 2.2 Error handling

`fromNative` (`errors.ts:100-135`) is careful: it accepts a facade-tagged error only when
`error.constructor === Error` (so a subclass cannot be hijacked), validates the category
against `Object.hasOwn`, and returns `null` rather than throwing on anything foreign. The
`SENDUST:{json}` envelope is the only string that crosses the boundary and it is parsed, not
regex-scraped. `indexOf` (rather than `startsWith`) tolerates napi prefixing its own text.

### 2.3 TypeScript findings

* **M-02 — double release of a pool transport.** `#run` releases inside `try` before mapping
  the result (`engine.ts:300`), then releases again in `catch` (`engine.ts:314`).
  `toSendResult` throws `SendustInternalError` for an unexpected status
  (`results.ts:12-24`), so a contract violation at the boundary releases the *same* transport
  twice: it is either handed to two waiters or pushed into `#idle` twice, and a later job can
  then drive two concurrent sends over one native transport (which the binding rejects with
  `TRANSPORT_BUSY`, a non-retryable code, so the symptom is a spurious refusal rather than a
  crash). Confirmed by code trace; the trigger requires the native layer to violate its own
  contract, so likelihood is low and the fix is trivial (release exactly once, in `finally`,
  against a nulled local).
* **M-09 — the fatal state is invisible.** After an auth failure `#fatal` is set
  (`engine.ts:427-441`) but `state` keeps reporting `"open"` (`engine.ts:155-157`).
* **M-12 — `tsc` typechecks 2 of 7 test files.** `tsconfig.json` includes
  `src/**/*.ts`, `test/types.test.ts`, `test/errors.test.ts` only (`ts/tsconfig.json:20`).
  Vitest transpiles without typechecking, so `transport.test.ts` (997 lines),
  `engine.test.ts`, `platform.test.ts`, `limits.test.ts` and `results.test.ts` are executed
  but never type-checked. A wrong argument type in the largest test file would pass CI.
* **L-05 (see §7)** — `to_napi` hand-builds JSON.

### 2.4 Loader and packaging

`loader.ts` is the only runtime-dependent module, and it earns that role: lazy, no static
`node:` import, `process.getBuiltinModule` guarded, bundled addon first then the per-platform
package, a missing module distinguished from a broken one (`isMissingModule`), and only
`error.code` surfaced so a dlopen message with absolute paths cannot leak
(`loader.ts:111-139, 196-200`). The `MODULE_URL_PLACEHOLDER` fallback for source-tree runs is
a pragmatic touch that keeps tests and examples working without a build step (`:160-174`).

---

## 3. Native binding quality

### 3.1 Strengths

* **Owned snapshots before any `await`** (ADR-0008): `Transport::send` converts the JS message
  synchronously into Rust-owned `String`s and `Vec<u8>`s (`napi/lib.rs:347, 464-515`), and the
  facade copies caller buffers before returning (`transport.ts:396`). Nothing borrows a JS
  object across a suspension point.
* **No finalizer, no GC dependence.** The runtime harness proves 50 created/closed transports
  and an abandoned open transport leave the next one usable (`tests/runtime/gc.mjs:25-49`).
* **Async work never blocks the event loop**: delivery runs in `tokio::spawn`
  (`napi/lib.rs:358`).
* **Cancellation maps to honesty**: abort → `cancelled_outcome()` → `unknown` if transmission
  began, `TRANSPORT_CLOSED` otherwise (`napi/lib.rs:399-404`).
* **Error conversion is total and escaped** (`lib.rs:537-588`).

### 3.2 Binding findings

* **H-02 — a failed delivery leaves the transport permanently `BUSY`.**

  **Severity:** HIGH. **Category:** native binding / reliability. **Confidence:** high on the
  mechanism (traced end to end); the trigger is a panic, and no panic path exists on any normal
  code path.

  **Evidence.** The slot is emptied before the work starts and refilled only on the success
  path:

  ```rust
  // sendust-napi/src/lib.rs:321-338 — the transport leaves the shared slot
  let transport = { let mut slot = self.inner.slot.try_lock()?;
                    match slot.take() { Some(t) => t, None => return Err(BUSY) } };

  // :357-370 — ownership moves into the spawned task
  let handle = tokio::spawn(async move { /* delivery */ (transport, outcome) });

  // :385 — joined here
  let joined = handle.await;

  // :392-406
  match joined {
      Ok((transport, outcome)) => { if OPEN { *slot.lock().await = Some(transport) } ... }
      Err(e) if e.is_cancelled() => /* progress-based outcome */,
      Err(_) => Err(to_napi(CoreError::new(ErrorCode::Internal))),
  }
  ```

  When the task ends with `Err(_)` — a panic inside the delivery, whether from core code or a
  dependency — the transport was dropped inside the task and the slot stays `None`. The next
  `send` reaches `slot.take()` → `None` → `TRANSPORT_BUSY` (`:330-337`).

  **Impact.** Three distinct problems:

  1. **No recovery.** Every later `send` on that transport fails. `close()` still returns
     `Ok`, so nothing in the public surface tells the caller the transport is dead; only
     constructing a new `Transport` restores service.
  2. **Wrong cause.** `TRANSPORT_BUSY` is documented as "another send is already active on
     this transport" — a transient contention signal. It is neither transient nor contention.
  3. **Not retryable.** `TRANSPORT_BUSY` is in neither `RETRYABLE_ERRORS` nor `FATAL_ERRORS`
     (`ts/src/policy.ts:56-64`), so the engine fails the message with a non-retryable error
     but keeps reporting `state: "open"`.

  **Why it is HIGH despite a low trigger.** No panic path was found in normal operation, so
  frequency is low. But the binding is the only place a panic can cross the FFI boundary into
  a permanently unusable object, and `#[napi(catch_unwind)]` is applied only to the
  constructor and `state()` (`lib.rs:169, 297`), not to the async methods — so a panic is
  *contained* (the process survives, which is good) at the cost of *losing the transport*
  (which is not). A future dependency panic would surface as a support ticket reading "Sendust
  says busy and never sends again".

  **Remediation (not applied).** Refill the slot on every exit, including the abnormal one,
  and represent "unusable" as its own state (`failed`) so the next send reports `INTERNAL` or
  `TRANSPORT_CLOSED` once rather than `BUSY` forever. A `cfg(test)` panic injection plus a
  follow-up-send assertion would lock it in.
* **L-05 — hand-built JSON.** `to_napi` formats a JSON object by string concatenation with a
  bespoke `escape` (`lib.rs:564-588`). It is correct today: quotes, backslashes, `\n`, `\r`,
  `\t` and other control characters are handled, and every interpolated value comes from a
  static table or the whitelisted details. But it is one added field away from a payload
  that `JSON.parse` rejects, and the failure would be a *silently unmapped* error (the facade
  returns `null` from `fromNative` and the caller sees a raw napi message). Using `serde_json`
  would remove the class, but it is **not** currently in the dependency graph — `Cargo.lock`
  contains no `serde` or `serde_json` entry at all, since `napi` is built without its serde
  feature (`Cargo.toml:57`). That trade belongs to the dependency policy, not to this fix. The
  cheaper remedy is a `escape_json` helper with a unit test that walks every byte `0x00`–`0x1F`
  plus `"` and `\`, so the escaper is proven rather than inspected.
* **`try_lock` on the slot** (`lib.rs:322`) can return a spurious `TRANSPORT_BUSY` if `close`
  happens to hold the slot; the window is small because the `active` mutex already serialises
  sends, and the code resets state before returning. Acceptable.
* **Four copies of "restore the slot"** (`lib.rs:325-336, 341-344, 350-354`) plus the normal
  path (`lib.rs:394-396`). A fifth caller would be easy to add without the restore.

---

## 4. Test quality (by inspection, not execution)

| Suite | Files | Lines | Character |
| --- | --- | --- | --- |
| Rust unit (in-module) | 25 modules | ~2,000 of the 8,452 | Boundary values, exact strings, precedence |
| Rust integration | 13 | 3,528 | Fake SMTP over real TLS, fault injection, session reuse, descriptor leaks, perf |
| TypeScript | 7 | 1,999 | Fake relay, lifecycle, pool behaviour, contract shape |
| Runtime harnesses | 4 | 526 | Per-runtime smoke, facade, full outcome contract, forced GC |

Notable strengths:

* A **second implementation** verifies DKIM: `tests/dkim.rs` re-derives the body hash and the
  signed header block from the emitted bytes using only RFC 6376 and the raw RSA primitive
  (`dkim.rs:1-7, 85-106`). A round-trip through the signer's own code would prove far less.
* The fake relay is a real TLS server built from committed fixtures, and `openssl verify`
  gates each fixture's intended failure mode in CI (`ci.yml:53-61`).
* The leak probes deliberately serialise a runtime to avoid counting a concurrent test's
  descriptors, with the CI failure that motivated it recorded in the comment
  (`leaks.rs:72-82`). That is a test written by someone who has been burned.
* `scripts/check-architecture.mjs` compares the TypeScript limit table with the Rust defaults,
  so the two copies cannot drift (`limits.ts:6-8`).

Gaps:

* **M-12** — most TypeScript tests are not type-checked.
* No test exercises an `AUTH` line near the 512-byte limit, which is why H-01 survives
  (`smtp_session.rs` asserts XOAUTH2 framing, not length).
* No test asserts that the `lifecycle` module is reachable from the shipped path, and no test
  would fail if `lifecycle` gained another 500 lines of unreachable code.
* No panic-injection test at the binding boundary, which is why H-02 survives.
* No `SIZE` negotiation test, because the feature does not exist (M-04).

---

## 5. Documentation accuracy

The docs are extensive and mostly precise — the ADR set is genuinely good, each with context,
alternatives and consequences. Four places where the documentation no longer describes the
code, all filed as findings:

| Document claim | Code reality | Finding |
| --- | --- | --- |
| `message/mod.rs:170-173`: "Callers run all stages through `validation::validate_in_order`" | No caller exists | M-06 |
| `error-model.md:107`: `PROTOCOL_REJECTED` "malformed or unexpected protocol failure that prevents a trustworthy outcome" | Nothing constructs it | M-11 |
| The error model and `ts/test/errors.test.ts` present `details.recipient` / `smtpEnhancedCode` as observable | Never set outside tests | M-10 |
| `types.ts:139`: host has "no whitespace" | Only control characters and emptiness are rejected | M-07 |

Two more are softer. `dkim/mod.rs:8-9` says the signature is "the signature every verifier
accepts, including relays that reject `rsa-sha1`" — true of the algorithm, but the audit
found no third-party verification in the tree; the ADR records a `dkimpy` check performed
outside CI, which cannot be re-run from the repository. And `limits.ts` claims
`check-architecture.mjs` compares the tables — plausible from the script's size (399 lines)
and the comment, but this audit did not read that script and does not assert it.

---

## 6. Findings — MEDIUM records in the shipped path

Two MEDIUM findings are recorded here in full because their evidence lives in the interaction
between layers rather than in one function. M-02 is recorded in §2.3, and the rest of the
MEDIUM set is in `SECURITY_REVIEW.md` §4 (H-01, M-07, M-08, M-09) and `ARCHITECTURE_REVIEW.md`
§5 (M-04, M-05, M-06, M-10, M-11) or §7 (M-13).

### M-01 — A cancellation during composition reports the previous message's recipients as `unknown`

**Severity:** MEDIUM. **Category:** reliability / outcome integrity. **Confidence:** high on
mechanism; the timing window was not measured.

**Evidence.** `Delivery::send` validates, reads file attachments, serialises the MIME tree and
signs it **before** the port is entered (`delivery/mod.rs:53-67`). The transport only resets
its progress record once the port is reached:
`self.reset_progress();` sits after the envelope is built and before the session is acquired
(`transports/smtp/mod.rs:355`). On the binding side, the spawned task aborts, and the abort
branch consults the shared `DeliveryProgress` retroactively:

```rust
// sendust-napi/src/lib.rs:399-404
Err(error) if error.is_cancelled() => match self.inner.progress.cancelled_outcome() {
    Some(outcome) => Ok(to_result(outcome)),
    None => Err(to_napi(CoreError::new(ErrorCode::TransportClosed))),
},
```

`cancelled_outcome` returns `Some(Unknown { .. })` whenever the recorded stage is at or past
`DataBody` and at least one recipient was accepted (`results/mod.rs:136-155`).

**What goes wrong.** For the **second and later** messages the sequence is: message A
completes, leaving `stage = DataBody` and A's accepted recipients in the record. Message B is
submitted; while B is still being validated or serialised, `close()` aborts it. `reset` has
not run, so the abort branch reads *A's* state and returns
`{ status: "unknown", recipients: [A's recipients] }` as B's result.

**Impact.** Two problems, both on the reporting path only:

1. **Wrong disposition.** B never reached the socket, so the honest answer is
   `TRANSPORT_CLOSED` (a definitive cancellation). The caller is told the message may have been
   transmitted and must now reconcile by hand — the exact ambiguity the outcome model exists to
   avoid.
2. **Address bleed.** B's result carries recipient addresses that came from A. If a single
   `Transport` serves messages for different tenants — a legitimate configuration, since the
   transport is per-relay and not per-message — one tenant's recipient address appears in
   another tenant's result object.

**Reproduction conditions.** Send A successfully (progress reaches `DataBody`, one recipient
accepted), then submit B with a large body or attachment and call `close()` before B's
composition finishes. The window is the cost of validation + file reads + base64 + DKIM signing
for that message — small for a text message, hundreds of milliseconds for a multi-megabyte one.

**Remediation (not applied).** Reset the progress record at the *start* of the send — in
`Transport::send` before spawning, or by giving each send its own `DeliveryProgress` — so a
cancellation before the transaction is unambiguously `TRANSPORT_CLOSED`. One line, plus a test
that aborts a second send during composition.

### M-03 — A session is retained even when the relay refuses `RSET`

**Severity:** MEDIUM (low frequency, disproportionate consequence). **Category:** SMTP session
lifecycle. **Confidence:** high.

**Evidence.** ADR-0022 promises a session "is kept only after a definitive outcome and only
when the relay answers the `RSET` that closes the transaction" (`smtp/mod.rs:229-233`). The
code checks whether the command *completed*, not what it answered:

```rust
// crates/sendust-core/src/transports/smtp/mod.rs:444-448
if needs_reset && session.reset().await.is_err() {
    return;
}
self.session = Some(session);
self.idle_since = Some(Instant::now());
```

`Session::reset` returns `Ok(Reply)` for any well-formed reply (`session.rs:40-44`), so a
`500 Command unrecognized` or `502` is `Ok` and the session is stored for reuse.

**What goes wrong.** `needs_reset` is set when an envelope was opened but `DATA` was never
completed, or when the `DATA` command was refused (`SessionState::aborted`,
`smtp/mod.rs:615-620`). In those states a relay that refuses `RSET` may still consider the
transaction open. The next message then issues `MAIL FROM` on a session with a pending
envelope, and a relay answering `503 Bad sequence of commands` produces:

* `mail_from` → non-2xx → `Verdict::Refused` → `rejected_outcome(...)` for **every**
  recipient (`smtp/mod.rs:466-479`);
* `SendOutcome::Rejected` with `response.code = 503`;
* the facade classifies `503` as permanent (`policy.ts:42-47, 107-109`), so the message is
  **never retried**.

**Impact.** A message that was never actually attempted is reported as permanently refused by
the relay, with a real relay response attached as evidence — the most persuasive possible form
of a wrong answer. The caller's queue drops it.

**Reproduction conditions.** Requires a relay that opens an envelope, has `DATA` abandoned or
refused, and then refuses `RSET`. Rare, because `RSET` is universally implemented — which is
why this is not HIGH.

**Remediation (not applied).** Treat a non-2xx `RSET` reply as a reason to drop the session,
which is what both the ADR and the module comment already say. One predicate in `retain`, plus
a fake-relay test that answers `500` to `RSET` and asserts the session is not reused.

## 7. Findings — LOW and INFORMATIONAL register

### LOW

| ID | Finding | Evidence | Impact |
| --- | --- | --- | --- |
| L-01 | The final `DATA` reply is read at `Stage::DataCommand`, not `DataBody` | `session.rs:94-97`; `smtp/mod.rs:549` marks `DataBody` first | A timeout awaiting the message disposition is annotated with the wrong stage; diagnostics only |
| L-02 | Two deadline clocks per send | `smtp/mod.rs:353` (run) vs `connection.rs:98` (connect) | `elapsed_ms` on an annotated error is measured from a different origin than the socket's bound |
| L-03 | `#concurrency` may exceed the effective connection count, and `stats.active` counts jobs still awaiting a transport | `engine.ts:280` incremented before `#acquire` resolves (`:288`); `:483` caps creation | `stats.active` can read 8 while 2 sockets exist; a monitoring consumer over-reads load |
| L-04 | `MAX_CONCURRENCY = 32` duplicates the `maxConcurrent` ceiling | `engine.ts:73`, `:483`; `transport.ts:210-216` | Two places to change; a raised `maxConcurrent` silently does nothing past 32 |
| L-05 | Hand-built JSON error payload | `napi/lib.rs:564-588` | One unescaped new field yields an unmappable error at runtime |
| L-06 | The normalised content type is discarded | `attachments/mod.rs:197` returns lowercase; `mime/mod.rs:262-266` re-emits the raw caller string | Canonicalisation is unimplemented; output case follows the caller. Harmless, misleading |
| L-07 | `Message-ID` uniqueness rests on the wall clock plus a content hash | `mime/mod.rs:348-357` | Two identical messages to one domain in the same nanosecond collide. Not reachable in practice, but it is not a UUID |
| L-08 | Multipart boundaries are the fixed pattern `sendust-boundary-NNNN` | `mime/mod.rs:420-430` | Correct because the collision check is load-bearing; predictability is irrelevant for MIME but the check must never be removed |
| L-09 | Custom caller headers are not DKIM-signed unless named | `dkim/mod.rs:41-52`; `DEFAULT_SIGNED_HEADERS` | A caller adding `X-Entity-Ref-ID` and assuming coverage is wrong; documented, but easy to misread |
| L-10 | `8BITMIME` is parsed and never used | `capabilities.rs:41, 65` | Harmless: every body is base64 (`mime/mod.rs:154`), so no `BODY=8BITMIME` is needed |
| L-11 | musl detection falls back to `gnu` when `process.report` is unavailable | `loader.ts:70-80` | A musl host without report support resolves the gnu package and fails to load; narrow, fails closed |
| L-12 | `sessionIdleMs: 0` means opposite things on each side | `transport.ts:284-290` accepts 0; `engine.ts:521` treats 0 as "never retire"; `smtp/mod.rs:266-269` treats idle 0 as "always expired" | No correctness break, but a documented range whose value 0 disables socket retirement while the core expires sessions — an ambiguous knob |
| L-13 | `escape` maps unrecognised control characters to a space | `napi/lib.rs:584` | Safe for JSON, lossy for the text. Only static strings and whitelisted details pass through, so unreachable today |
| L-14 | Attachment/descriptor validation runs three times per send | `delivery/mod.rs:53`, `:85` (via `attachments::validate`), `mime/mod.rs:62` | Each pass allocates a lowercased-name vector (`attachments/mod.rs:102-118`). Negligible cost with ≤100 recipients; the real cost is that a future rule must be added in one place or behaviour diverges |
| L-15 | No liveness check on a pooled session | `smtp/mod.rs:383-399` | A session the relay closed between sends produces an error on the next message's first command. The facade retries `PROTOCOL_CONNECTION_RESET` (`policy.ts:60`), so facade callers see a transparent retry; a direct core caller sees the error |

### INFORMATIONAL

| ID | Observation |
| --- | --- |
| I-01 | No CRITICAL finding; no exploitable vulnerability identified |
| I-02 | The STARTTLS plaintext-injection defence is present and correct (`connection.rs:213-215`) |
| I-03 | Error redaction is tested exhaustively across all 35 codes (`errors/mod.rs:376-396, 459-471`) |
| I-04 | The attachment reader's `O_NOFOLLOW` + post-open `fstat` sequence is TOCTOU-correct and tested against a real FIFO (`attachments/mod.rs:276-320, 721-733`) |
| I-05 | DKIM output is verified by an independent implementation (`tests/dkim.rs`) |
| I-06 | No DANE / MTA-STS / DNSSEC; WebPKI-only relay trust (documented post-MVP) |
| I-07 | No certificate pinning; `tlsRootCertificate` is additive only |
| I-08 | DKIM key and bearer token exist as immutable JS strings the library cannot wipe |
| I-09 | No SMTPUTF8, DSN, PIPELINING or CHUNKING; all declared out of scope |
| I-10 | `SmtpConfig` derives `Debug` and prints host/port while `TransportOptions` has no `Debug` at all — a defensible asymmetry, undocumented |
| I-11 | `SmtpTransport`'s `Debug` prints `host`, `port`, `session: bool` and `served`, but never credentials or the reuse policy (`smtp/mod.rs:284-293`); worth keeping that way deliberately |
| I-12 | Rate limiting bounds throughput, not connection churn: `messagesPerSecond` defaults to 0 (unlimited) with `maxConcurrent` 8, so a burst can open 8 simultaneous AUTH sessions against one relay (`engine.ts:73`, `transport.ts:64-74, 246-252`). Correct by design; a relay that bans on connection rate will punish the default configuration |

---

## 8. Maintainability assessment

**Good:** module boundaries; naming (`begin_transaction`, `transit_state`, `remote_instruction`
style consistency); error taxonomy; deadline model; test intent; comment density targeted at
*why*.

**Weak:** four contracts with no consumer (M-05, M-06, M-10, M-11); one capability parsed and
ignored (M-04); documentation that describes a design the code has partly outgrown; a
binding that has absorbed lifecycle policy the architecture assigns elsewhere.

**Suitable for long-term open-source development?** Yes, with one condition: the project must
stop letting declared-but-unwired abstractions accumulate. Each individual one is small; the
aggregate is what erodes trust in the docs. The remedy is cheap (delete or wire), and the
roadmap puts it early for that reason.
