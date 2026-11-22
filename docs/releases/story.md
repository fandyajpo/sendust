# The Sendust story

**From an empty folder to `1.0.1`.**
Dates are real; every claim links to the record that backs it.

Sendust is a Rust email delivery engine with a TypeScript face, for Node.js, Bun,
and Deno. It sends through SMTP, reports what the relay said, and never claims a
message reached a mailbox. This is how it got here.

## 1. Before any code

The project did not start with a library; it started with constraints. P00 wrote
down what Sendust would be and refused to be — not an SMTP server, not a hosted
service, not a queue, not a provider integration
([discovery brief](../discovery/brief.md)). P01 surveyed the ground and chose an
existing engine, `lettre`, to avoid owning the protocol
([ADR-0002](../architecture/decisions/0002-lettre-over-custom-smtp.md)). P02 fixed
the architecture that everything later had to obey: two Rust crates, a thin
binding, a runtime-neutral facade, verification always on, one send at a time,
no retries ([ADR-0006](../architecture/decisions/0006-lifecycle-single-send.md)).

Those decisions were deliberately narrow. Three of them were later reversed, and
each reversal has a record explaining why.

## 2. The unpublished era (P03–P18)

Everything a user would eventually touch was built and verified while **no
package existed on npm at all**. The repository could always be cloned and built;
nothing could be installed.

| Phase | What it added | Why it mattered later |
| --- | --- | --- |
| P03 | Two-crate workspace, pinned toolchain, architecture checks, CI | Every later claim rests on these gates |
| P04 | Error taxonomy, validation, addresses, MIME, caps, timeouts | The contract users read in the docs |
| P05 | Node-API binding | The bridge from Rust to the three runtimes |
| P06 | **The native SMTP engine** ([ADR-0017](../architecture/decisions/0017-native-smtp-engine.md)) | `lettre` was replaced; Sendust owned its protocol |
| P07 | Deterministic MIME composition | Byte-for-byte reproducible messages |
| P08 | Attachments, opt-in filesystem access | Security surface kept explicit |
| P09 | The TypeScript facade and error classes | The API became stable |
| P10 | Lifecycle and bounded close | `close()` became a promise you can keep |
| P11 | Security audits | Injection, ownership, TLS, redaction |
| P12 | QA harness and traceability | Evidence instead of assertions |
| P13 | Performance budgets and harnesses | Numbers before claims |
| P14 | Five native targets, per-platform packages | One install, the right binary |
| P15 | Documentation set | Users could follow it without asking |
| P16 | Release engineering: hashes, provenance, a publish gate | Publishing became a reviewed act |
| P17 | **First publication**: `0.1.0-rc.1` | 2026-11-18, the npm `next` tag |
| P18 | Post-MVP intake, research, backlog | The program closed with no open defects |

The shape of that era matters: a release candidate was assembled and **deliberately
not published** until the gate approved it. When `0.1.0-rc.1` finally shipped on
2026-11-18 it was hash-verified from the registry and its behaviour contract
passed on all three runtimes.

At that point Sendust did exactly one thing at a time: a second concurrent
`send()` threw `TRANSPORT_BUSY`, every message opened a new TLS session, and
nothing was retried. That was the approved design, not an accident.

## 3. The pivot: from sender to engine

Then came a request that changed the product's centre of gravity: make Sendust
autonomous — accept a thousand sends, queue them, pool connections, respect the
provider, retry only what is safe. Nodemailer was the reference point.

The first step was not code but an [audit](../audits/2026-11-18-autonomous-delivery-audit.md)
that traced the real path: `send()` → native slot → new TCP connection → new TLS
handshake → EHLO → AUTH → one transaction → `QUIT`. It found no correctness
defect and one structural fact: **every message paid a full handshake**, and the
`TRANSPORT_BUSY` string came from a single-flight guard in the binding.

[ADR-0021](../architecture/decisions/0021-autonomous-delivery-engine.md) then
split the problem in two. The native core kept its guarantee — one session, one
transaction — and the facade gained the policy: a bounded queue, a pool of native
transports, adaptive concurrency, a token bucket for rate, and retries with
jitter that stop at `unknown` and at any `5xx`. That shipped as part of `1.0.0`.

[ADR-0022](../architecture/decisions/0022-session-reuse.md) went further and made
the native layer keep its authenticated session, reset it with `RSET`, and reuse
it — but never after an error, and never after an `unknown` disposition, because
a reused session must not be able to duplicate a message.

## 4. `1.0.0`: first stable release

`1.0.0` was published on 2026-11-19, tagged `v1.0.0`, and verified from the
registry: the tarball matched the approved facade hash byte for byte, a clean
install pulled only the facade and the matching platform package, and the
behaviour contract passed on Node.js, Bun, and Deno
([release notes](1.0.0.md), [gate record](../gates/release-1.0.0.md)).

Two things went wrong on the way, and both are recorded rather than tidied away:

- the publish job hardcoded `--tag next`, so `1.0.0` landed under `next` while
  `latest` still pointed at `0.1.0-rc.1`. The workflow now reads the dist-tag
  from `release/<version>/approved.json`, and a manual `dist-tag` job moves a tag
  on an already-published version instead of republishing it.
- a benchmark seemed to show Sendust re-establishing sessions. It was a stale
  harness with the default 100-message cap; the corrected run showed one session
  per pooled connection, and a test now pins it.

Then, for the first time, a real message: `smtp.gmail.com:465`, an app password,
`accepted`, `250 2.0.0 OK` ([real-relay probe](../qa/real-relay-probe.md)).

## 5. The real relay tells the truth

A loopback relay never pushes back, so it flatters a client. The owner ran 100
messages at concurrency 5 against Gmail, Sendust against Nodemailer: 100/100
accepted on both, throughput within 7% — and a p95 of **11.9 s against 5.3 s**.

That shape was diagnostic. At ~4 s per message and five lanes, both clients were
near the relay's ceiling; the difference was *when* they reached it. Sendust
started with one lane and added one per eight successes, so most of a burst ran
under-provisioned and messages queued. Nodemailer fills its pool immediately.

Two fixes followed ([analysis](../qa/real-relay-optimization.md)): concurrency
now rises to the work already waiting, and `RSET` is no longer sent after a
completed transaction, because a finished `DATA` leaves the session idle anyway.
Backoff kept its teeth: any temporary failure still halves the level and starts a
cooldown, and a `421` now drops the session rather than reusing a channel the
relay is closing.

That is `1.0.1`.

Then an audit of the whole delivery path found one thing the release notes had
implied and the code had not done: an idle session stopped being **reused** after
thirty seconds, but nothing closed it, so a service that sent one message and
stayed up held an authenticated relay connection — and its TLS state — until the
next send or `close()`. `1.0.2` releases it, and looks at the field properly:
[the competitive landscape](../research/competitive-landscape.md) compares
Sendust with Nodemailer, Upyo, EmailJS and the hosted-API SDKs, including the
places Sendust currently loses.

That is `1.0.2`, published on 2026-11-21; it is what a bare
`npm install @sendust/ts` installs.

## 6. Version timeline

| Version | Date | What changed |
| --- | --- | --- |
| — | until 2026-11-17 | Built and verified; nothing published |
| `0.1.0-rc.1` | 2026-11-18 | First publication, npm `next`; one send at a time |
| `1.0.0` | 2026-11-19 | Queue, pool, adaptive concurrency, bounded retries, session reuse |
| `1.0.1` | 2026-11-21 | Demand-driven pool; one fewer round trip per message |
| `1.0.2` | 2026-11-21 | Idle sessions released; options validated before the native transport |

## 7. What Sendust is now

- One transport per process, not per request: the engine queues what callers
  hand it and keeps the pool at the level the work needs.
- One authenticated SMTP session serves many messages, and is dropped the moment
  its state is uncertain.
- Only transient failures are retried; `unknown` never is, because a retry after
  an uncertain `DATA` can deliver twice.
- `accepted` means the relay took responsibility. There is no `delivered`.
- TLS verification cannot be turned off. There is no plaintext mode.

## 8. What it is not, by decision

No persistent queue, no scheduler, no worker process, no queue adapters, no
delivery or open tracking, no provider-specific quota tables — those are
[rejected with reasons](../planning/post-mvp-backlog.md), because they belong to
the application or cannot be observed honestly.

## 9. What comes next

- Provider policy profiles: documented limits only, with `421` as a first-class
  signal rather than something inferred from `4xx`.
- A re-run of the owner's Gmail benchmark against `1.0.1`, to replace the
  projection in the analysis with a measurement.
- Trusted publishing before the npm bypass-2FA token deadline in January 2027.
- Deferred until someone needs them: SMTPUTF8, `AbortSignal`, Windows arm64 and
  Linux musl.
