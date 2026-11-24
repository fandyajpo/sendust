# Security Review — Sendust 1.2.1

Attack surface, controls verified in source, confirmed findings, model risks, and residual
exposure. Static review only; no exploit was executed.

---

## 1. Verdict

**No exploitable vulnerability was identified.** The library's own threat model
(`docs/discovery/threat-model.md`) is largely implemented as written. What remains is a small
set of *model* risks — places where the trust boundary is the caller, not the wire — which
should be documented rather than fixed.

The most valuable thing found is not a security defect at all: **H-01**, an AUTH framing
limit that breaks XOAUTH2 for large tokens, is filed here because it is the finding with the
highest operational impact and it hides behind a security-shaped error code.

---

## 2. Attack surface

| Surface | Reachable from | Trust |
| --- | --- | --- |
| Relay host, port, security mode | caller | semi-trusted config |
| Credentials (password / bearer token) | caller or secret store | secret |
| DKIM private key | caller | secret |
| Message: addresses, subject, headers, bodies | often untrusted application data | **untrusted** |
| Attachment content (buffers) | caller | untrusted bytes |
| Attachment **paths** | caller | trusted only by convention |
| TLS trust anchors | caller | trust decision |
| Relay replies and EHLO lines | **network attacker / hostile server** | **untrusted** |

The last row matters most: every byte the relay sends is parsed by hand-written code.

---

## 3. Controls verified in source

### 3.1 Transport security

| Control | Evidence | Assessment |
| --- | --- | --- |
| No plaintext mode | `SmtpSecurity` has exactly `ImplicitTls` and `StartTls` (`smtp/mod.rs:44-51`) | Correct |
| STARTTLS never downgrades | `starttls()` fails when not advertised (`client/connection.rs:204-207`) | Correct |
| AUTH only over encryption | `if !self.encrypted { return Err(AuthUnsupported) }` (`connection.rs:371-373`) | Correct |
| Verification cannot be disabled | No `dangerous_*` API; `client_config` always sets roots + hostname checking (`client/tls.rs:22-40`); the module header states the invariant (`tls.rs:1-5`) | Correct |
| Trust store | bundled Mozilla roots + caller anchors (`tls.rs:24-30`) | Correct, additive only |
| TLS floor | `with_safe_default_protocol_versions` with `ring` provider (`tls.rs:33-35`) | Correct (1.2+) |
| Hostname verified | `ServerName::try_from(host)`; IP literals check IP SANs (`tls.rs:42-44`) | Correct |
| STARTTLS plaintext injection | after `220`, `if !self.buffer.is_empty() { return Err(protocol_error()) }` (`connection.rs:213-215`) | **Correct** — the CVE-2011-0411 class is closed |
| Capabilities not trusted pre-TLS | `self.capabilities = Capabilities::default()` then re-EHLO (`connection.rs:238-239`) | Correct |
| Certificate fixtures | `tests/fixtures/tls` + `openssl verify` gates for valid, self-signed, expired, wrong-host, incomplete-chain, malformed (`.github/workflows/ci.yml:53-61`) | Strong |

### 3.2 Injection

| Vector | Control | Evidence |
| --- | --- | --- |
| SMTP command injection (address) | atom set, rejects `< > , space`, control chars, ≤320 | `command.rs:136-145`, `smtp/mod.rs:677-687` |
| SMTP command injection (EHLO) | printable ASCII only, ≤255 | `smtp/mod.rs:155-167`, `command.rs:64-67` |
| AUTH response framing | control/space rejection, ≤12,288 | `command.rs:129-134` (but see H-01) |
| Header injection (name) | field-name chars only, ≤78 | `message/mod.rs:19-31` |
| Header injection (value) | all controls except TAB rejected | `message/mod.rs:51-65` |
| Header injection (subject) | same validator applied | `message/mod.rs:190` |
| Header override / BCC hiding | reserved-name list includes `bcc`, `from`, `return-path`, `received`, `content-type`… | `message/mod.rs:130-145, 196-198` |
| Duplicate header shadowing | case-insensitive single-instance rule | `message/mod.rs:201-204` |
| Address/display-name injection | control chars rejected before parsing; display names quoted+escaped | `address/mod.rs:112-117, 186-195` |
| MIME part injection (filename) | basename only, no control chars, ≤255 | `attachments/mod.rs:146-158` |
| MIME part injection (content type) | exactly one `/`, both sides token chars | `attachments/mod.rs:188-198` |
| DATA transparency | dot-stuffing per RFC 5321 §4.5.2 | `proto/transcript.rs:7-24` |
| Reply parsing (hostile server) | strict 3-digit codes, CRLF required, ≤1000/line, ≤64 KiB/reply, mismatched multiline codes rejected | `proto/response.rs:163-241` |
| DKIM tag injection | domain/selector grammar; header names validated; restricted set rejected | `dkim/mod.rs:96-147, 401-422` |

**Assessment: closed.** Every producer re-validates at its own boundary rather than trusting
an earlier one, which is what makes this robust to future refactoring. Multiple
`#[test]` functions in `command.rs:220-243`, `message/mod.rs:443-458`, `address/mod.rs:367-386`
and `attachments/mod.rs:430-452` assert the specific injection strings.

### 3.3 Secrets

| Control | Evidence | Assessment |
| --- | --- | --- |
| Credentials redacted in `Debug` | `SmtpCredentials` prints `<redacted>` for both fields (`smtp/mod.rs:108-115`) | Correct |
| Credentials never in errors | `Error` is a code + whitelisted details; no path from a password to any field | Correct |
| DKIM key redacted | `Debug` prints `<redacted>` (`dkim/mod.rs:240-249`) | Correct |
| Error redaction tested across all codes | iterates `ErrorCode::ALL` asserting no `password`/`secret`/`token`/`body`/`private key` (`errors/mod.rs:459-471`) | Strong |
| Attachment paths never echoed | `errors_never_carry_the_path` (`attachments/mod.rs:760-770`); loader exposes only a code, not a dlopen message (`ts/src/loader.ts:135-138, 196-200`) | Correct |
| AUTH bytes never logged | No logging facility exists in the core at all | Correct by absence |

### 3.4 Filesystem

`read_attachment` (`attachments/mod.rs:265-320`), matching ADR-0014:

1. `validate_path` — absolute, ≤1024 bytes, no control chars.
2. `symlink_metadata` — reject symlink, non-regular *before* opening (so a FIFO cannot block).
3. `open` with `O_NOFOLLOW | O_NONBLOCK` on unix (line 291-295).
4. `metadata()` on the **opened descriptor** and require a regular file (line 302-305).
5. `take(cap + 1).read_to_end()` — a file that grows past the cap is rejected, not truncated
   (line 309-320).

This is a correct TOCTOU-resistant sequence, and it is tested against a real FIFO created with
`libc::mkfifo` (`attachments/mod.rs:721-733`), a character device, a symlink and a directory.

### 3.5 Availability / abuse

| Control | Evidence |
| --- | --- |
| Every read/write bounded by a deadline | `client/connection.rs:181-188, 252-258, 286-289` |
| Connect budget covers DNS+TCP+TLS+greeting+EHLO as one window | `connection.rs:103-134` |
| Single total deadline caps all operations | `config/mod.rs:233-240` |
| Recursion/loop bounds | reply line and total size caps; RCPT loop bounded by recipient cap |
| Queue bounded by count and bytes | `engine.ts:188-194` |
| Auth failure stops the transport instead of hammering | `policy.ts:64, 131-133`, `engine.ts:427-441` |

---

## 4. Findings

### H-01 — `AUTH` is capped at 512 bytes, so XOAUTH2 cannot carry a normal bearer token

**Severity:** HIGH (functional, not a vulnerability). **Category:** SMTP protocol / auth.
**Confidence:** high on mechanism, medium-high on field impact.

**Evidence.**

```rust
// crates/sendust-core/src/transports/smtp/proto/command.rs:13
pub const MAX_COMMAND_BYTES: usize = 512;

// :69-77  — initial response accepted up to 12,288 characters
Self::Auth { mechanism, initial_response } => match initial_response {
    Some(response) => { validate_token(response, 12_288)?;
                        format!("AUTH {} {response}", mechanism.as_str()) }

// :103-105 — then the whole line is capped at 512
if line.len() + 2 > MAX_COMMAND_BYTES { return Err(protocol_error()); }
```

`auth_xoauth2_initial` (`command.rs:124-127`) builds
`base64("user=<mailbox>\x01auth=Bearer <token>\x01\x01")`; base64 inflates by 4/3. The
`AUTH XOAUTH2 ` prefix costs 13 bytes, the `user=`/`auth=Bearer ` framing about 17, leaving
roughly **330 bytes of raw token** inside a 512-byte line.

**Impact.** Gmail app-scoped OAuth tokens are ~200 bytes and fit — which is why the XOAUTH2
addition works in the tests. Microsoft 365 / Graph access tokens are JWTs of typically
800–2,000+ characters. They do not fit, so the send aborts *before dialling*:

* the caller receives `VALIDATION_HEADER_INJECTION` — a code whose documented meaning is
  "header or address contains control characters", which points the user at the wrong
  subsystem entirely;
* `code == "VALIDATION_HEADER_INJECTION"` is not in `RETRYABLE_ERRORS` (`policy.ts:56-61`) and
  not in `FATAL_ERRORS`, so the engine fails the message permanently without stopping the
  transport — every message in the batch fails identically.

**Reproduction.** `DkimSigner` aside, no relay is needed: constructing
`Command::Auth { mechanism: XOAuth2, initial_response: Some(auth_xoauth2_initial("user@contoso.test", &"t".repeat(400))) }`
and calling `to_bytes()` returns `Err(ValidationHeaderInjection)`. Condition: total line
above 510 bytes, i.e. raw token above ~330 bytes.

**Why the suite misses it.** The XOAUTH2 tests use short synthetic tokens
(`crates/sendust-core/tests/smtp_session.rs` asserts payload framing, not length).

**Remediation (for the roadmap, not applied).** Separate the *command* limit from the *AUTH
payload* limit: RFC 4954 §4 allows an initial response beyond the 512-octet command line, and
servers are required to accept 1,228+ octets. The 12,288 validator already encodes the right
intent; the 512 check must not apply to the `Auth` and `AuthResponse` variants. Also map an
over-long credential to `VALIDATION_INVALID_OPTION`, not to an injection code.

### M-07 — `host` accepts embedded whitespace, contrary to the documented contract

**Severity:** MEDIUM. **Category:** validation / contract. **Confidence:** high.

```rust
// crates/sendust-core/src/transports/smtp/mod.rs:132-137
if host.trim().is_empty() || host.chars().any(char::is_control) || port == 0 {
    return Err(Error::new(ErrorCode::ConfigInvalidRelay) ...);
}
```

`"re lay.example"` and `"relay .example"` are accepted, and so is `" a b "` (it is not
empty after trimming). The TypeScript contract says otherwise: *"Relay hostname; non-empty,
**no whitespace** or control characters"* (`ts/src/types.ts:139`).

**Impact.** Not a security hole — `TcpStream::connect` fails and the send reports
`PROTOCOL_CONNECTION_RESET` after dialling, which misdescribes a configuration error as a
network event. A caller debugging a typo in a host string gets a reset instead of
`CONFIG_INVALID_RELAY`. In an environment where the host comes from an env var or a config
service, this is the kind of mismatch that costs an on-call hour.

**Remediation.** Reject any whitespace in `host` at construction, matching the published
contract. One predicate, one test.

### M-08 — Attachment paths have no root constraint once file access is enabled

**Severity:** MEDIUM (model risk, by design). **Category:** security / trust boundary.
**Confidence:** high on the mechanism; exploitation requires caller-side misuse.

**Evidence.** `validate_path` requires only: non-empty, ≤1024 bytes, no control characters,
`Path::is_absolute()` (`attachments/mod.rs:244-256`). With
`allowFileAccess: true`, any absolute path to any regular readable file on the host is
readable into a message. ADR-0014 states this explicitly as the policy — there is no
allowlist, no base directory, no `chroot`, and no capability model.

**Attack scenario.** An application exposes a "send with attachment" endpoint and maps a
request field to `attachment.path`. The library's protections (no symlink, regular file
only, capped size) are all satisfied by `/etc/shadow` on a misconfigured container, or by
`/app/.env`. The email leaves through the application's own trusted relay, from its own
domain, and passes SPF/DKIM — it is not spam-filtered on the way out. Result: authenticated
exfiltration using the application's mail reputation.

**Why this is MEDIUM, not HIGH.** The library's boundary is the caller; the option is
opt-in, named `allowFileAccess`, defaults to `Disabled` (`attachments/mod.rs:23-30`), and
ADR-0014 justifies the design. Reading a caller-supplied path is the feature.

**Remediation.** Documentation, not code: state in the guide and on `FileAttachment.path`
that the value must **never** be derived from untrusted input, and that enabling file access
grants "read any regular file this process can read". If a hardening option is ever wanted,
a `baseDirectory` with a canonicalised prefix check would be additive and backwards
compatible.

### M-09 — A single authentication failure stops the whole transport, silently

**Severity:** MEDIUM. **Category:** reliability / operational. **Confidence:** high.

`AUTH_FAILED` and `AUTH_UNSUPPORTED` are `fatal` (`policy.ts:64, 131-133`); the engine then
stores the error and rejects every queued message and every scheduled retry with it, and
`submit` rejects new work immediately (`engine.ts:185-187, 427-441`). The reasoning is sound
— bad credentials do not heal by waiting — but two consequences are undocumented:

1. `stats.state` remains `"open"` while every send fails (`engine.ts:155-157` reads
   `#state`, which `#abortWith` never changes). A health check keyed on
   `stats.state === "open"` reports healthy forever.
2. There is no reset. Credential rotation in-process (a token function that starts returning
   a valid token) cannot recover the transport; the caller must construct a new one.

**Remediation.** Surface the fatal state in `stats` (for example `state: "failed"` or a
`fatal: true` flag) and document the recovery path.

---

## 5. Threats considered and found adequately handled

| Threat | Position |
| --- | --- |
| MITM on port 587 | STARTTLS required and verified; no opportunistic fallback |
| MITM stripping STARTTLS from EHLO | `starttls()` fails the send (`connection.rs:204-207`) |
| MITM injecting commands after `STARTTLS` | Pending-plaintext check (`connection.rs:213-215`) |
| Malicious relay sending a 100 MB reply | `MAX_LINE_BYTES` 1000, `MAX_REPLY_BYTES` 64 KiB (`response.rs:10-12`) |
| Malicious relay sending bare-LF or mismatched multiline codes | Rejected (`response.rs:170, 189-193`) |
| CRLF in any caller string | Rejected at every boundary (§3.2) |
| Secret in an error message or a log | No logging; whitelisted details; exhaustive redaction test |
| Path traversal via filename | Basename only (`attachments/mod.rs:150`) |
| FIFO / device blocking the process | `O_NONBLOCK` + pre-open type check + post-open `fstat` |
| Symlink swap between check and read | `O_NOFOLLOW` closes the race for that open |
| Oversized attachment exhausting memory | Per-file, aggregate and wire caps, all enforced before the port |
| Prototype pollution through `fromNative` | `Object.hasOwn` guard (`errors.ts:95-97`); class selected from a fixed table |
| Path disclosure through the addon loader | Only `error.code` is exposed (`loader.ts:196-200`) |

---

## 6. Supply chain

| Aspect | Observation |
| --- | --- |
| Rust pins | Every direct dependency is exactly pinned (`Cargo.toml:15-63`), `Cargo.lock` committed, CI uses `--locked` (`ci.yml:44,48,67`) |
| Rust dev-dependencies | `mail-parser` 0.11.9 only, test-only, never shipped |
| npm runtime dependencies | **None.** The facade has no `dependencies` at all; the platform packages are `optionalDependencies` injected by `prepare-publish.mjs` |
| npm dev-dependencies | `@biomejs/biome` 2.6.7, `typescript` 7.0.2, `vitest` 5.1.0 — pinned, not published |
| Install-time scripts | No `postinstall`, no `node-gyp`, no compile step: the tarball ships `dist/` + `sendust.node` (`ts/package.json:50-54`) |
| Publication | OIDC trusted publishing, `id-token: write`, `NODE_AUTH_TOKEN` pinned empty with a guard, SLSA provenance attestation (`ci.yml:492-543`) |
| Secrets in CI | `NPM_TOKEN` used only by the dispatch-only `dist-tag` job (`ci.yml:597-609`) |
| Secret scanning | `scripts/check-secrets.mjs` runs with a self-test in CI (`ci.yml:92-96`) |
| License | Dual MIT / Apache-2.0, consistent across both crates and the npm manifest |

**Gap:** no automated advisory scanning (no `cargo-audit`, no `npm audit`) in the workflow.
Given the exact pins and the near-zero dependency count this is a low-risk omission, but
"pinned" and "unaffected by a recent advisory" are different claims and only the first is
currently checked.

---

## 7. Informational

* **I-06** No DANE, MTA-STS or DNSSEC support. Relay trust is WebPKI-only. Explicitly
  post-MVP; worth stating in the security guide so users do not assume otherwise.
* **I-07** No certificate pinning. `tlsRootCertificate` *adds* an anchor; it cannot restrict
  the set. A caller who wants pinning cannot express it. ADR-0018 documents the additive
  model; the limitation deserves a sentence in the user-facing docs.
* **I-08** The DKIM private key and any bearer token necessarily exist as immutable
  JavaScript strings (passed as `privateKey` / `accessToken`), whose memory the library cannot
  wipe. The Rust side copies them into `String`s and parses the key into `RsaPrivateKey`;
  `zeroize` is present in the dependency graph (`Cargo.lock:1149`) but this audit did not
  verify which types are wiped on drop. Callers should treat the key as resident for the
  process lifetime.
* **I-10** On Deno the addon needs `--allow-ffi`, and file attachments need `--allow-read`.
  This narrows Deno's sandbox and should be in the compatibility notes.
* **I-12** No rate limiting toward a *single* relay beyond the token bucket and concurrency
  ceiling; a misconfigured `messagesPerSecond: 0` with 32 concurrency can open 32
  simultaneous AUTH sessions. Acceptable, but a relay that bans on connection churn will
  punish the default configuration rather than a hostile one.
