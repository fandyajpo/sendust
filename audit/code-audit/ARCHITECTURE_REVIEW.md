# Architecture Review — Sendust 1.2.1

Scope: module responsibilities, execution flow, dependency relationships, and architectural
concerns. Derived entirely from source. Static review only; nothing was executed.

---

## 1. The intended architecture

The project documents a modular monolith in exactly two crates (ADR-0016): domain modules,
application orchestration, one narrow outbound port (`DeliveryPort`), and one adapter
(`SmtpTransport`). The binding is described as owning "JavaScript conversion, owned-memory
snapshots, asynchronous task and lifecycle bridging, and error/result mapping only"
(`crates/sendust-napi/src/lib.rs:3-5`).

That is a good architecture. The review below judges how faithfully it is realised.

---

## 2. Module responsibilities, as implemented

| Module | Lines | Declared responsibility | Actually does | Verdict |
| --- | --- | --- | --- | --- |
| `address` | 409 | Mailbox parse, IDNA, injection rejection | Exactly that; ASCII-only local part, strict IDNA, control-char rejection | Clean |
| `attachments` | 782 | Policy, descriptor validation, bounded reads | Plus filename normalisation, content-type validation, duplicate detection | Clean |
| `config` | 474 | `Limits`, `Timeouts`, `SendDeadlines` | Lower-only caps, absolute deadlines, honest timeout classification | Clean |
| `delivery` | 440 | Orchestrate one send | Validate → load → serialize → sign → cap → port. No network, no retry | Clean |
| `dkim` | 702 | RFC 6376 signing | Own canonicalisation, tag rendering, key parsing, restriction checks | Clean, but see §5.4 |
| `errors` | 507 | Append-only taxonomy | Macro-generated codes, whitelisted details, redaction tests | Clean |
| `lifecycle` | 273 | `open → closing → closed`, one-send reservation, drain | **Nothing in the shipped path** | **Dead (M-05)** |
| `message` | 545 | Message model, header newtypes | Plus reserved-header and duplicate policy, inline stage ordering | Clean but see M-06 |
| `mime` | 1,270 | Composition, encoding, serialisation, wire accounting | Plus date/Message-ID generation, boundary search, preflight | Clean |
| `ports` | 37 | The single outbound seam | Exactly that, with a blanket impl for `&mut T` | Clean |
| `results` | 320 | Outcomes and per-recipient truth | Plus `DeliveryProgress` for cancellation classification | Clean |
| `transports/smtp` | 1,983 | The only adapter | Session reuse, transaction, outcome mapping, error annotation | Clean |
| `validation` | 207 | Stage ordering, single source of precedence | **Nothing in the shipped path** | **Dead (M-06)** |
| `sendust-napi` | 589 | Conversion + bridging | Also implements the lifecycle state machine and cancellation policy | **Boundary breach (M-05)** |

---

## 3. Execution flow (traced)

### 3.1 Happy path, one message, warm pool

```
ts/transport.ts:550   Transport.send           → engine open? build native record
ts/engine.ts:181      DeliveryEngine.submit    → queue bound, deadline stamp
ts/engine.ts:265      #pump                    → concurrency, rate limit, #acquire
ts/engine.ts:299      transport.send           ─┐ cross FFI
napi/lib.rs:308       Transport::send          │ phase check, slot reservation, spawn
napi/lib.rs:464       build_message            │ rebuild typed message (Mailbox, Header)
core/delivery:50      Delivery::send           │ validate → read files → serialize_at
core/delivery:59      DkimSigner::sign         │ optional, on composed bytes
core/delivery:67      port.deliver             │ DeliveryRequest { message, wire }
smtp/mod.rs:352       SmtpTransport::run       │ deadlines, envelope, session
smtp/mod.rs:383       acquire_session          │ reuse if warm, healthy, under caps
smtp/mod.rs:455       transact                 │ MAIL → RCPT* → DATA → body
session.rs:66-97      Session::*               │ one command per call, bounded
client/connection:245 Connection::command       │ write, flush, then read reply
proto/command.rs:62   Command::to_bytes         │ injection-proof serialisation
proto/transcript:7    encode                    │ dot-stuffing + terminator
proto/response.rs:163 ReplyBuilder::push_line    │ strict, bounded parser
napi/lib.rs:517       to_result                 ─┘ outcome → JS record
ts/results.ts:31      toSendResult              → frozen public shape
```

Nothing in this path allocates per-character, opens a runtime of its own, or performs a
retry. The core never builds a runtime: it uses the caller's timer and reactor.

### 3.2 Error path

Errors are annotated once, at the adapter, with relay context that the session may already
have supplied (`smtp/mod.rs:745-776`): host, port, stage, elapsed, SMTP code. Codes are never
rewritten, only enriched. The binding renders a JSON payload and the facade re-hydrates it
into a typed class (`napi/lib.rs:537-573`, `ts/src/errors.ts:100-135`). This is a clean
two-way contract for a boundary that cannot carry Rust types.

### 3.3 Cancellation path

`close()` sets the phase, aborts the spawned task, and classifies the result from
`DeliveryProgress` (`napi/lib.rs:399-404`). If transmission had begun, the caller gets
`unknown`; otherwise `TRANSPORT_CLOSED`. The *idea* is right — never invent an outcome — and
the core supports it well (`results/mod.rs:136-155`). The stale-window defect in M-01 is in
where the reset happens, not in the model.

---

## 4. Dependency relationships

```
      ts/src/*            (no node: imports; globalThis only)
         │  dynamic require through a single seam
      ts/src/loader.ts    (platform package resolution, libc detection)
         │
      sendust-napi        (napi 3.16.2, tokio)
         │
      sendust-core        (idna, base64, tokio, tokio-rustls, rustls, webpki-roots,
         │                 rustls-pki-types, rsa, sha2, libc on unix)
      dev-only: mail-parser (MIME conformance), tokio rt+macros
```

Assessment:

* **Direction is correct and enforced.** The core cannot reach a JS runtime; CI runs
  `scripts/check-architecture.mjs` and compiles the core alone before anything else.
* **Fan-in is low.** `ports` has one implementor; `results` is consumed by two modules;
  there are no cycles.
* **`tokio` features are minimal** (`time`, `sync`, `net`, `io-util`) — the core does not
  pull in multi-thread or macros. The binding adds the runtime. This is the right split.
* **Two crates, not one, is justified**: the core is testable without Node headers, proven in
  CI (`cargo check -p sendust-napi` is a separate step).

---

## 5. Architectural concerns

### M-05 — The lifecycle is enforced twice, and the tested copy is the dead one

**Evidence.** `core/src/lifecycle/mod.rs` defines `Transport` with a `Mutex<State>` +
`Condvar`, `begin_send() -> SendPermit`, `begin_close()`, `wait_closed(Duration)`, and its own
unit tests for every transition, including drain and drain timeout (`lifecycle/mod.rs:163-272`).
Exporting it (`lib.rs:29`) is its only production footprint: a repository-wide search for
`LifecycleState`, `begin_send`, `wait_closed` and `SendPermit` returns only its own
declaration, its own tests, and the re-export. The binding instead implements the same
policy with `AtomicU8` `phase` (`OPEN/CLOSING/CLOSED`), a `tokio::sync::Mutex<Active>` and a
`Notify` (`napi/lib.rs:20-22, 142-157, 308-453`).

**Why it matters.** The `open → closing → closed` contract, the "one active send" rule and
the bounded drain are *lifecycle* policy, which ADR-0016 assigns to the core. Two facts
follow. First, the unit tests the project cites as lifecycle evidence (for example
`docs/gates/phase-06.md:66`, `docs/qa/traceability-matrix.md:97`) exercise code that no
longer runs; the behaviour they "prove" is proven for a module the product does not use.
Second, the copy that *does* run has no direct unit tests — only end-to-end coverage through
the TypeScript suite and the runtime harnesses. A change to the binding's phase handling is
guarded by integration tests alone.

**Not a bug.** The shipped behaviour was traced and is correct. This is a
maintainability-and-evidence defect: it inflates apparent coverage and invites drift.

**Options.** (a) Delete `lifecycle` and record that the binding owns the state machine —
honest, minimal. (b) Move the phase machine into the core and have the binding drive it —
restores ADR-0016, larger change. (c) Keep both and add a test that asserts they agree —
worst option; two implementations still drift.

### M-06 — Validation ordering is declared, not enforced

`validation::validate_in_order` and `ValidationStage` claim to be "the single source of
truth for error precedence" (`validation/mod.rs:1-8`). A search finds no caller: only
`message/mod.rs:173` refers to it in a doc comment ("Callers run all stages through
`validation::validate_in_order`"), and `OutgoingMessage::validate` runs the stages inline
(`message/mod.rs:174-214`). The inline order happens to match the enum, so behaviour is
correct; the abstraction is decorative. Either wire it or delete it — the current state
means the precedence table can be edited without any test failing.

### M-10 / M-11 — Declared contract with no producer

`ErrorDetails` has `recipient` and `smtp_enhanced_code` accessors (`errors/mod.rs:214-227,
249-259`); `annotate` forwards them when present (`smtp/mod.rs:765-773`), but nothing ever
sets them outside tests (`errors/mod.rs:384-386, 481-483`). The RFC 3463 parser exists and
works (`response.rs:85-100`) but `Reply::enhanced()` is only read by its own tests. The
public error surface therefore advertises `details.recipient` and
`details.smtpEnhancedCode` — typed in TypeScript, tabled in the error model and the TS error
tests — that a caller can never observe. `PROTOCOL_REJECTED` (`errors/mod.rs:133`) is in the
same position: declared, documented in `docs/architecture/error-model.md:107` and
`docs/guides/errors-security-operations.md:102`, asserted in `ts/test/errors.test.ts:59`, and
never constructed by any code path — definitive refusals are outcomes, not errors, by design.

### M-04 — A parsed capability that is not acted on

`Capabilities` parses `SIZE` and `8BITMIME` (`capabilities.rs:38-41`) and exposes them
(`:61-67`). `size()` is read only in a unit test. Nothing compares the composed wire length
against the advertised limit, and `MAIL FROM` carries no `SIZE=` parameter
(`command.rs:89-92`). The practical consequence: a caller can hand Sendust a 30 MiB message
for a relay that advertises 10 MiB, and finds out from the relay's refusal — or from a
connection dropped mid-body — rather than from a clear local limit.

### 5.5 Correct boundaries worth preserving

* **`DeliveryPort`** is genuinely narrow (one method, boxed future) and has a blanket
  `&mut T` impl (`ports.rs:33-37`), which is what lets the binding hand a transport to
  `Delivery` without surrendering ownership. This is the right seam for a future second
  transport.
* **`SendDeadlines`** is a value, not a timer. Nothing else could be tested deterministically
  without sleeping.
* **`Envelope::of`** re-validates addresses at the protocol boundary rather than trusting
  `Mailbox` (`smtp/mod.rs:677-687`), with a comment explaining that a hand-built
  `OutgoingMessage` must not surface as an injection error. Defence in depth done for the
  right reason.

---

## 6. Data-flow and trust boundaries

```
untrusted input            boundary                        control
────────────────────────────────────────────────────────────────────────────────
relay config        →  SmtpConfig::new / with_hello_name   control chars, printable ASCII,
                                                            port ≠ 0 (but see M-07)
addresses           →  Mailbox::parse                     ASCII local, strict IDNA, no
                                                            control chars, ≤254
custom headers      →  HeaderName/HeaderValue             atom chars, no CR/LF/NUL/tab
subject             →  HeaderValue::parse                same
attachment name     →  normalize_filename                basename only, no control chars
content type        →  normalize_content_type            exactly token/token
attachment path     →  validate_path                      absolute, ≤1024, no control
body text           →  normalize_body                    newline normalisation only
DKIM domain/select  →  is_domain / is_selector           DNS label grammar
DKIM key            →  parse_key                         PEM, not ENCRYPTED, 1024–4096 bit
relay replies       →  ReplyBuilder                      ≤1000/line, ≤64 KiB/reply, strict
```

Every row is a real, implemented check, each at its own boundary. The architecture's
security argument rests on this table and it holds.

---

## 7. Scalability and resource shape

* **One engine per transport**, with a pool bounded by `maxConnections` ≤ 32 and an
  absolute concurrency ceiling of 32 (`engine.ts:73, 483`). No unbounded fan-out exists.
* **Queue is doubly bounded** (count and estimated bytes), and the byte estimate deliberately
  under-counts (`transport.ts:449-458`) so it fails open on acceptance rather than rejecting a
  message the core would take.
* **No streaming anywhere.** `serialize` builds the whole wire in one `Vec`, base64 bodies are
  produced 57 bytes at a time into it, and the signer copies again. Peak memory for one send is
  the message plus its normalised form plus the encoded form plus the signed copy — a multiple
  of the payload, within a hard 30 MiB wire cap and a 20 MiB attachment cap. Bounded, but
  M-13 records that a 32-way concurrent burst multiplies it.

---

## 8. Assessment

**Strengths.** Correct dependency direction; one narrow port; authoritative error taxonomy;
deterministic deadlines as values; defence-in-depth validation at every boundary; per-target
CI that builds and loads its own binary.

**Weaknesses.** Contract duplication (M-05, M-06) and unsatisfied declarations (M-10, M-11,
M-04) mean the architecture documents describe an intention the code has partly outgrown.
For an open-source library whose selling point is a small, honest surface, the fix is
deletion or wiring — never a third copy.

**Maturity:** the structural architecture is better than the average 1.x native Node addon.
The *evidence discipline* — tests pointing at dead modules, docs asserting unproducible codes
— is where it lags, and that is a cheaper problem to fix than a structural one.
