# lettre 0.11.23 — Source Validation Report

**Research date:** 2026-11-13  
**Status:** source inspection complete; compile and runtime probes remain  
**Pinned source:** [lettre v0.11.23](https://github.com/lettre/lettre/tree/v0.11.23)

## 1. Decision summary

Use lettre 0.11.23 with its public low-level asynchronous SMTP connection API, not its high-level send transport, because Sendust's contract needs the individual RCPT replies. This is still lettre's SMTP implementation: Sendust does not write its own SMTP parser, TLS stack, AUTH exchange, or dot-stuffing codec.

The selected API surface is:

- `lettre::transport::smtp::client::AsyncSmtpConnection`
- `connect_tokio1`, `starttls`, `auth`, `command`, `message`, `quit`, `abort`
- `lettre::transport::smtp::commands::{Mail, Rcpt, Data}`
- `lettre::transport::smtp::Error::{status,is_response,is_tls,is_timeout}`
- `lettre::transport::smtp::response::{Response,Code,Severity}`

Source locations:
- [async_connection.rs](https://github.com/lettre/lettre/blob/v0.11.23/src/transport/smtp/client/async_connection.rs)
- [commands.rs](https://github.com/lettre/lettre/blob/v0.11.23/src/transport/smtp/commands.rs)
- [error.rs](https://github.com/lettre/lettre/blob/v0.11.23/src/transport/smtp/error.rs)
- [response.rs](https://github.com/lettre/lettre/blob/v0.11.23/src/transport/smtp/response.rs)
- [Cargo.toml](https://github.com/lettre/lettre/blob/v0.11.23/Cargo.toml)
- [tls.rs](https://github.com/lettre/lettre/blob/v0.11.23/src/transport/smtp/client/tls.rs)

## 2. Public API findings

### 2.1 Connection and negotiation

The source exposes `AsyncSmtpConnection::connect_tokio1(server, timeout, hello_name, tls_parameters, local_address)`. It creates the async stream, reads the server greeting, sends EHLO, and returns a connection.

Important timeout finding: the supplied timeout is applied to TCP connect attempts, but DNS lookup and the greeting/EHLO work occur outside that inner TCP timeout. TLS upgrade is also not fully covered by that inner timeout. Sendust MUST wrap the entire connect/negotiation future in its own outer connect deadline and the absolute total deadline. The inner lettre timeout remains a useful additional TCP bound.

For implicit TLS, pass rustls `TlsParameters` to `connect_tokio1`.

For STARTTLS, connect without TLS, then require `can_starttls()` and call `starttls(parameters, hello_name)`. If STARTTLS is not advertised or negotiation fails, abort. Never use opportunistic TLS or continue in plaintext. AUTH occurs only after TLS is established.

### 2.2 Per-recipient RCPT outcomes

`AsyncSmtpConnection::command` is public and accepts a `Display` command. `Rcpt::new(address, vec![])` is public. The returned `Response` exposes `code()`; a negative SMTP response is returned as `lettre::transport::smtp::Error`, whose public `status()` returns the numeric response code.

Therefore Sendust can issue one RCPT command per recipient, record explicit positive/negative responses, and then issue one DATA transaction for the accepted recipients. This resolves the prior ADR-0005 uncertainty at the source/API level. It does not yet prove behavior against a live local fake relay; P06-003 must do that.

Do not treat any generic `Ok(Response)` as success. `read_response()` returns `Err(Error)` for negative 4xx/5xx replies, while 2xx and 3xx are both returned as `Ok(Response)` because `Response::is_positive()` includes both positive completion and positive intermediate. Sendust must check the exact expected SMTP code class and command-specific response:
- greeting/EHLO: validate the expected 2xx completion;
- AUTH: require final 235 (334 is handled internally as a challenge by lettre's auth method);
- MAIL: require 2xx completion;
- RCPT: accept only a 2xx completion; map explicit 4xx/5xx errors to recipient rejection; an unexpected 3xx is a protocol error, not acceptance;
- DATA command: require exactly 354 before transmitting message bytes; explicit 4xx/5xx is a known rejection;
- final DATA: accept only 2xx; explicit 4xx/5xx is a definitive rejection; an unexpected 3xx is not a final acceptance and must be treated as unknown after message transmission.

If no recipient receives a positive RCPT reply, do not issue DATA. If at least one recipient is accepted, submit exactly one DATA message to that accepted-recipient envelope. Recipients refused at RCPT remain rejected even if the accepted subset later succeeds.

### 2.3 Final DATA ambiguity

`message()` does **not** issue the SMTP DATA command. Sendust must first call `command(Data)` and require exactly 354. Only then call `message()`, which calls `message_iter()`, dot-stuffs the content, writes the DATA terminator, and reads the final response. A definitive negative final response is represented as a lettre response error with a status code and maps to Sendust `rejected`.

A network/write/read error from `message()` does not always reveal whether the DATA terminator reached the relay. Because the API does not expose a public “terminator definitely not written” checkpoint, Sendust must conservatively map any indeterminate error once message transmission has begun to `unknown`. A response error carrying an explicit 4xx/5xx final code remains a known `rejected` outcome. Do not retry. This is intentionally conservative and avoids duplicate mail.

If a positive final DATA response is received, the message is accepted. A subsequent QUIT/cleanup failure must not rewrite an already confirmed accepted result.

### 2.4 Error inspection

The SMTP error type exposes `status()`, `is_response()`, `is_client()`, `is_transient()`, `is_permanent()`, `is_timeout()`, `is_tls()`, and `is_transport_shutdown()`. The response `Code` converts to `u16`. Use structured methods, never parse `Display` strings.

### 2.5 Connection cleanup

The connection exposes `quit()` and `abort()`. On an indeterminate DATA transmission failure, close/abort the socket without attempting a protocol-level QUIT that could obscure the outcome. On known pre-DATA failures, abort and return the corresponding operational error or explicit rejection. After a confirmed final 2xx, QUIT is best-effort; cleanup failure cannot revoke relay acceptance.

## 3. Exact Cargo feature proposal

Disable lettre defaults so its default pool and native-tls backend are not compiled:

```toml
lettre = {
  version = "=0.11.23",
  default-features = false,
  features = [
    "smtp-transport",
    "tokio1-rustls",
    "ring",
    "webpki-roots",
  ],
}
```

Source-level feature graph:
- `smtp-transport` enables SMTP primitives and Tokio networking/time support when Tokio is selected.
- `tokio1-rustls` enables `tokio1`, `rustls`, and `tokio1_rustls`.
- `ring` enables rustls' ring provider feature.
- `webpki-roots` enables the bundled root set.
- `default-features = false` avoids lettre's default `pool` and `native-tls` features.
- The separate lettre `builder` feature is not required if Sendust owns MIME serialization and does not use lettre's message builder. Revisit only if P07 proves a specific builder primitive is needed.

This is a source-reviewed feature graph, **not a compiled dependency graph**. A clean Cargo build and `cargo tree -e features` remain mandatory at P03-002.

## 4. TLS policy

Use `TlsParameters::builder(host)`, explicitly choose `CertificateStore::WebpkiRoots`, keep `set_min_tls_version(TlsVersion::Tlsv12)`, and never call:
- `dangerous_accept_invalid_certs(true)`
- `dangerous_accept_invalid_hostnames(true)`

The builder defaults both bypass flags to false and defaults the minimum to TLS 1.2. The public API must not expose these dangerous switches. Source review confirms the secure path; five local certificate fixtures and a source audit remain required at P06-002/P11-002.

STARTTLS must be required, not opportunistic. Implicit TLS is preferred when configured.

## 5. Timeout and stage observability

Sendust can instrument coarse boundaries around public calls:
- DNS/connect/greeting/EHLO/TLS setup (one outer connect deadline)
- AUTH command exchange
- MAIL command
- each RCPT command
- DATA command
- DATA body + terminator + final reply
- QUIT/abort cleanup

It cannot honestly promise finer internal timing for individual socket reads/writes hidden inside lettre. Use an outer timeout around each coarse future and one absolute total deadline. If the total deadline fires during message transmission and the final disposition is not observed, return `unknown`. Do not invent a more precise stage.

## 6. Security and correctness caveats

- The high-level transport is not selected because it does not provide the per-RCPT visibility needed by the public contract.
- lettre's low-level API still owns SMTP parsing, command formatting, TLS, AUTH, dot-stuffing, and response parsing.
- Exact response codes must be checked by Sendust; lettre treats 3xx as “positive” at the generic response layer.
- No custom SMTP command parser or handwritten dot-stuffing is allowed.
- One connection and one SMTP transaction per send; no pool, internal queue, worker, retry, scheduler, or persistence.

## 7. Validation still required

| Probe | Expected evidence | Owner |
| --- | --- | --- |
| Clean feature build | `cargo check` and `cargo tree -e features` with no pool/native-tls | P03-002 |
| RCPT mixed responses | One 250 and one 550; accepted subset receives one DATA; no second message | P06-003 |
| DATA responses | 354 required; final 250 accepted; final 451 rejected; dropped connection after terminator unknown | P06-004 |
| TLS fixtures | Self-signed, expired, wrong hostname, incomplete chain, malformed cert all fail closed | P06-002/P11-002 |
| Timeouts | DNS/connect/TLS/greeting/AUTH/MAIL/RCPT/DATA/final response bounded at coarse operation limits | P10-002 |
| Runtime | napi-managed Tokio works on tested Node versions; close/GC/exit races are bounded | P05-003/P14-004 |

**Conclusion:** the architecture can rely on lettre's public lower-level API for per-recipient outcomes and SMTP orchestration. The design is sufficiently specified to enter P02 review, with compilation and executable integration tests retained as hard downstream gates.