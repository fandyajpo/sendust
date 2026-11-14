# QG-06 — Native SMTP Engine Phase Record

**Decision:** APPROVED — project owner review, 2026-11-14  
**Phase:** P06 — SMTP engine  
**Gate:** PHASE-06  
**Prepared:** 2026-11-14  
**Branch:** `feat/p06-004-native-adapter`  
**Evidence:** local Rust, checker, and runtime commands below, plus the branch CI run.

This record is the review package for PHASE-06. The implementation was written
by an agent, so the approval in section 6 is the required human review step, not
a self-approval.

## 1. Task evidence

| Task | Status | Evidence |
| --- | --- | --- |
| P06-001 | COMPLETED | `proto/command.rs` (EHLO, STARTTLS, AUTH PLAIN/LOGIN, MAIL, RCPT, DATA, RSET, NOOP, QUIT; injection rejection; 512-byte cap), `proto/response.rs` (3-digit code validation, multiline builder, 1000-byte line and 64 KiB reply caps, RFC 3463 enhanced status), `proto/capabilities.rs` (STARTTLS, AUTH, SIZE, 8BITMIME), `proto/transcript.rs` (dot-stuffing). 9 unit tests in the codec and parser |
| P06-002 | COMPLETED | `client/connection.rs` (Tokio TCP, greeting, EHLO, implicit TLS, required STARTTLS with re-EHLO, bounded reads, stage-accurate timeouts) and `client/tls.rs` (rustls with `ring`, WebPKI roots plus validated PEM anchors, TLS 1.2 floor, no bypass). `tests/smtp_client.rs`: 6 tests |
| P06-003 | COMPLETED | `client/connection.rs::authenticate` (PLAIN with an RFC 4616 initial response, LOGIN with a bounded 334 exchange, refused before TLS, `AUTH_UNSUPPORTED`, `AUTH_FAILED` with the relay code) and `session.rs` (open, starttls, authenticate, MAIL, RCPT, DATA, body, QUIT). `tests/smtp_session.rs`: 6 tests |
| P06-004 | COMPLETED | `transports/smtp/mod.rs` runs the delivery on the native `Session`; `SmtpConfig`, `SmtpTransport` and `SmtpSession` are unchanged; `annotate` restores relay context; `tests/smtp_faults.rs` grew to 18 cases; `tests/differential_lettre.rs` compared six matrix cases against the lettre oracle before the oracle was removed on approval; `scripts/check-architecture.mjs` rejects lettre in production dependencies and in any `src/` file |
| P06-005 | COMPLETED | This record |

## 2. Acceptance checklist

| Check | Result | Evidence |
| --- | --- | --- |
| The SMTP engine is implemented in Sendust | PASS | `transports/smtp/{proto,client,session.rs,mod.rs}` contain the codec, parser, capabilities, TLS/STARTTLS, AUTH, session and delivery mapping |
| No third-party SMTP engine is a production dependency | PASS | `cargo tree -e normal -p sendust-core` and `-p sendust-napi` report zero lettre lines; `check-architecture` fails if lettre appears either way |
| TLS verification is mandatory and every certificate failure closes before DATA | PASS | `every_bad_certificate_fixture_fails_before_mail` (self-signed, expired, wrong-host, incomplete-chain, malformed anchor) and `an_untrusted_certificate_fails_closed`; no MAIL or DATA in any trace |
| STARTTLS is required and never downgraded | PASS | `starttls_is_required_and_fails_closed_without_the_extension`, `refuses_starttls_when_not_advertised`, `upgrades_with_starttls_and_rehandshakes` (two EHLOs); no plaintext fallback exists in the client |
| Recipient policy matches ADR-0005 and never fans out | PASS | `all_recipients_refused_never_sends_data`, `mixed_recipient_replies_share_one_data_transaction` (one MAIL, two RCPT, one DATA), `channel_close_at_rcpt_matches_the_lettre_oracle` (421 stops the envelope) |
| Accepted, rejected and unknown results are honest; no retry exists | PASS | `a_refused_sender_is_a_rejected_outcome`, `explicit_final_data_rejection_is_a_rejected_outcome`, `missing_final_reply_after_data_is_unknown`; the adapter issues one DATA transaction and returns |
| The differential fault matrix passes with exact command traces | PASS | `tests/differential_lettre.rs` ran before approval: six cases asserted identical statuses, recipients and envelope verbs for both backends. The oracle was then removed, and `tests/smtp_faults.rs` keeps the 18 native cases |
| No pooling, queue, worker or background delivery | PASS | One session per send; no pool, queue or spawn exists in the adapter; `cargo tree` shows no pool feature |
| Credentials never appear in errors or logs | PASS | `SmtpCredentials` has a redacting `Debug`; `auth_failure_is_an_error_and_never_mails` asserts the password is absent and only code 535 is retained |
| The public result contract is unchanged | PASS | `results` and `errors` are untouched; the Node-API and TypeScript suites pass unchanged |

## 3. Fault matrix

Every row is a test in `crates/sendust-core/tests/smtp_faults.rs` against the
loopback fake SMTP/TLS harness; nothing contacts a real relay.

| Required case | Test | Result |
| --- | --- | --- |
| Success | `accepts_a_single_recipient_transaction` | PASS — accepted, one MAIL/RCPT/DATA, body transmitted |
| All RCPT refused | `all_recipients_refused_never_sends_data` | PASS — rejected, no DATA |
| Mixed RCPT | `mixed_recipient_replies_share_one_data_transaction` | PASS — accepted first, rejected second, one DATA |
| RCPT 421 after one accepted | `channel_close_at_rcpt_matches_the_lettre_oracle` | PASS — envelope stops, no DATA |
| Explicit final 451 | `explicit_final_data_rejection_is_a_rejected_outcome` | PASS — rejected with code 451, no retry |
| Missing final reply after DATA | `missing_final_reply_after_data_is_unknown` | PASS — unknown, one DATA |
| AUTH 535 | `auth_failure_is_an_error_and_never_mails` | PASS — `AUTH_FAILED`, status 535, no MAIL/DATA |
| No advertised mechanism | `auth_without_a_supported_mechanism_is_unsupported` | PASS — `AUTH_UNSUPPORTED`, no MAIL/DATA |
| Self-signed certificate | `every_bad_certificate_fixture_fails_before_mail` | PASS — `TLS_CERTIFICATE`, no MAIL/DATA |
| Expired certificate | same test | PASS — `TLS_CERTIFICATE` |
| Wrong-host certificate | same test | PASS — `TLS_CERTIFICATE` |
| Incomplete chain | same test | PASS — `TLS_CERTIFICATE` |
| Malformed trust anchor | same test | PASS — `VALIDATION_INVALID_OPTION` at configuration |
| Connection refused | `connection_refused_is_a_protocol_error` | PASS — `PROTOCOL_CONNECTION_RESET` with the dialled port |
| Greeting reset | `greeting_reset_is_a_protocol_error` | PASS — `PROTOCOL_CONNECTION_RESET` at `connection_setup` |
| DATA-command reset | `data_command_reset_is_a_protocol_error` | PASS — `PROTOCOL_CONNECTION_RESET` at `data_command`, no body written |
| Abrupt pre-final close | `missing_final_reply_after_data_is_unknown` | PASS — unknown |
| connect 11 s stall | `connect_stall_is_a_connect_timeout` | PASS — `TIMEOUT_CONNECT` at `connection_setup` with lowered budgets |
| command 31 s stall | `command_stall_is_an_operation_timeout` | PASS — `TIMEOUT_OPERATION` at `mail` |
| total 121 s stall | `total_stall_is_a_total_timeout` | PASS — `TIMEOUT_TOTAL` |
| close 126 s drain | `lifecycle::tests::close_drain_times_out_without_the_active_send_finishing` | PASS — `TIMEOUT_CLOSE` with a lowered drain budget |
| STARTTLS absent | `starttls_is_required_and_fails_closed_without_the_extension` | PASS — `TLS_HANDSHAKE`, no MAIL/DATA |
| Intermediate reply where a completion is required | `an_intermediate_rcpt_reply_is_a_protocol_error` | PASS — `PROTOCOL_UNEXPECTED_RESPONSE` at `rcpt` |

The three stall rows use budgets lowered proportionally to the approved maximums
(see `lowered()` in the test file) so the same classification path is proven
without a five-minute suite. The close-drain row proves the lifecycle budget the
same way. The shapes, not the wall-clock values, are what the gate claims.

## 4. Parser security review

- **Bounded input.** A reply line is capped at 1000 bytes, a complete reply at
  64 KiB, and a command line at 512 bytes; the reader keeps at most one line in
  its buffer and drains it as it parses. A larger line or reply is a
  `PROTOCOL_UNEXPECTED_RESPONSE` error, never an allocation or a hang.
- **Smuggling resistance.** Reply codes are validated as exactly three digits in
  200–599, continuations must repeat the same code, and the separator must be a
  space or a hyphen. Command arguments reject control characters, spaces and the
  envelope delimiters `<`, `>`, `,`, so no CRLF, second command or header-like
  sequence can be injected. DATA bodies are dot-stuffed and terminated with
  `\r\n.\r\n` by `proto/transcript.rs` alone.
- **STARTTLS downgrade resistance.** A configured STARTTLS requires the
  advertisement; the client fails with `TLS_HANDSHAKE` when it is missing, never
  falls back to plaintext, requires an empty read buffer before upgrading, and
  re-sends EHLO after the handshake. Verification is always enabled: `client/tls.rs`
  builds the rustls config from WebPKI roots plus validated PEM anchors and
  exposes no bypass.
- **Truncation handling.** EOF while reading a reply is
  `PROTOCOL_CONNECTION_RESET`; an incomplete multiline reply is a parse error;
  a missing final DATA reply after the terminator is `unknown`, never `accepted`.
- **Redaction.** Errors carry a code, a fixed message and whitelisted details
  only. Credentials are never rendered: `Debug` redacts both halves and the AUTH
  error keeps only the numeric code.
- **Stage accuracy.** Every read and write carries the stage that issued it, and
  the connect window bounds greeting, EHLO and TLS, so a stall is reported at
  `connection_setup`, `auth`, `mail`, `rcpt`, `data_command`, `data_body` or
  `cleanup` as observed.

## 5. Executed evidence

Run on macOS with the pinned toolchain (`rustc 1.100.0`), Node 22.23.5,
Bun 1.3.14, Deno 2.9.8:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
node scripts/check-architecture.mjs
node scripts/check-docs.mjs
node scripts/check-secrets.mjs
npm run build:native
npm --workspace ./ts run test
node tests/runtime/smoke.mjs
bun tests/runtime/smoke.mjs
deno run --allow-ffi --allow-read --allow-net tests/runtime/smoke.mjs
cargo tree -e normal -p sendust-core
```

Result: 114 core unit tests, 6 differential-oracle tests, 9 harness self-tests,
6 connection tests, 18 fault-matrix tests and 6 session tests pass; every checker
passes; `cargo tree -e normal` reports no lettre in either crate. CI runs the
Rust workspace, the binding, the TypeScript matrix, and the Node/Bun/Deno
smokes on the branch.

## 6. Decision

**APPROVED.** The project owner reviewed this record and its evidence on
2026-11-14 and approved the PHASE-06 gate. The implementation was written by an
agent, so this approval is the required human review step, not a self-approval:
the reviewer is the project owner and the evidence is the CI run on the branch
plus the local commands above. On approval the lettre differential oracle and its
dev-dependency were removed, since parity was proven and ADR-0017 forbids a
third-party engine returning to the tree.

Approval covers P06 only. It does not approve P07 or any later gate; a failed
compile, test, checker or runtime result on `main` reopens this gate.

## 7. Deferred (recorded, not claimed)

- A live relay send through the native engine succeeded on 2026-11-14: the project
  owner ran `cargo run -p sendust-core --example send_email` and the relay accepted
  the message. The Node-API entry point is covered by the CI runtime smoke; a live
  run through `node examples/send_email.mjs` is still worth doing before release.
- Public TypeScript API, MIME headers (`Date`, `Message-ID`), attachments,
  runtime/platform matrices, and observability are P07–P18 scope.
- The 11 s / 31 s / 121 s / 126 s cases are proven with proportionally lowered
  budgets; wall-clock defaults are configuration, not gate evidence.
