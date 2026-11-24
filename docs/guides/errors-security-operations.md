# Errors, security, and operations

**Status:** user and operator guide for the MVP. Codes and classes match
[the error model](../architecture/error-model.md) and `ts/src/errors.ts`.

## 1. Two channels: results and errors

A send ends in exactly one of two ways:

- **A result** for a relay outcome: `accepted`, `rejected`, or `unknown`.
- **A `SendustError`** for anything that prevents a trustworthy outcome:
  validation, configuration, limits, TLS, authentication, protocol, timeouts,
  a closed transport, or an internal fault.

Branch on `result.status` and `error.code`. Never parse message text: it is not a
contract, and it deliberately omits credentials, bodies, and paths.

```js
import { SendustError } from "@sendust/ts";

try {
  const result = await transport.send(message);
  // accepted / rejected / unknown
} catch (error) {
  if (error instanceof SendustError) {
    // error.code, error.category, error.details
  }
} finally {
  await transport.close();
}
```

Every error extends `SendustError` and carries:

- `code` — stable string, append-only within a major version.
- `category` — one of `validation`, `configuration`, `limit`, `tls`,
  `authentication`, `protocol`, `timeout`, `closed`, `busy`, `internal`.
- `details` — a whitelisted object (see section 3).

There is one class per category: `SendustValidationError`,
`SendustConfigurationError`, `SendustLimitError`, `SendustTlsError`,
`SendustAuthenticationError`, `SendustProtocolError`, `SendustTimeoutError`,
`SendustClosedError`, `SendustBusyError`, `SendustInternalError`.

## 2. Catalog

### Validation — fix the input

| Code | Cause | Caller action |
| --- | --- | --- |
| `VALIDATION_INVALID_OPTION` | Unknown or malformed option/message key | Fix the call; typos are rejected, never ignored |
| `VALIDATION_INVALID_TIMEOUT` | Timeout outside its range, or an inconsistent ordering | Lower into range; see the [limits guide](lifecycle-limits.md) |
| `VALIDATION_MISSING_FROM` | No `from` | Provide one |
| `VALIDATION_MISSING_RECIPIENT` | No recipient across `to`/`cc`/`bcc` | Provide at least one |
| `VALIDATION_INVALID_ADDRESS` | Unparseable address | Fix the address |
| `VALIDATION_NON_ASCII_LOCAL_PART` | Local part outside ASCII | Use an ASCII local part; SMTPUTF8 is post-MVP |
| `VALIDATION_INVALID_HEADER` | Illegal field name, reserved name, or invalid content type | Fix the header or attachment metadata |
| `VALIDATION_HEADER_INJECTION` | CR/LF or control characters in a header, address, or filename | Remove them |
| `VALIDATION_INVALID_ATTACHMENT` | Missing filename, unsupported attachment shape | Fix the attachment |

### Configuration — fix the setup

| Code | Cause | Caller action |
| --- | --- | --- |
| `CONFIG_INVALID_RELAY` | Empty/invalid host or port | Fix the relay |
| `CONFIG_INVALID_DKIM_KEY` | The DKIM key does not parse, or is not a 1024–4096 bit RSA key | Export the key as PKCS#1 or PKCS#8 PEM and check its size; the key is read at `createTransport` |
| `CONFIG_UNSUPPORTED_SECURITY` | Security mode outside `tls`/`starttls` | Use a supported mode; there is no plaintext |
| `CONFIG_UNSUPPORTED_AUTH` | Auth mechanism outside `plain`/`login`/`xoauth2`, or a forced mechanism the relay does not advertise | Use a supported mechanism, or omit `auth.method` |
| `CONFIG_FILE_ACCESS_DISABLED` | Path attachment without opt-in | Set `attachments.allowFileAccess` deliberately, or use buffer attachments |
| `CONFIG_ATTACHMENT_UNREADABLE` | Missing, unreadable, or non-regular file | Fix the path; only bounded regular files are read |
| `CONFIG_NATIVE_UNAVAILABLE` | The addon is missing or cannot load on this platform | Install the build for this OS/CPU; no fallback build exists |

### Limits — reduce the payload

| Code | Cause | Caller action |
| --- | --- | --- |
| `LIMIT_RECIPIENTS` | More than 100 recipients | Split the message |
| `LIMIT_ATTACHMENT_BYTES` | One attachment over 10 MiB | Shrink or link out |
| `LIMIT_ATTACHMENT_TOTAL` | Raw attachments over 20 MiB | Reduce the set |
| `LIMIT_WIRE_BYTES` | Serialized message over 30 MiB | Reduce attachments or body size |

Limits cannot be raised; lowering them is allowed.

### TLS — never bypassed

| Code | Cause | Caller action |
| --- | --- | --- |
| `TLS_CERTIFICATE` | Certificate or hostname verification failed | Fix the relay certificate, hostname, or (for a private relay) add its CA through `tlsRootCertificate` |
| `TLS_HANDSHAKE` | Negotiation failed | Check protocol/cipher support at the relay |
| `TLS_PROTOCOL` | Unsupported TLS version or cipher | Fix the relay configuration |

There is no option to disable verification and no plaintext fallback. A
`tlsRootCertificate` **adds** anchors to the bundled public roots; it can never
remove them or turn verification off.

### Authentication and protocol

| Code | Cause | Caller action |
| --- | --- | --- |
| `AUTH_FAILED` | Relay rejected the credentials | Fix credentials, then build a new transport; it is `failed`, so it never sends again |
| `AUTH_UNSUPPORTED` | Relay offered no supported mechanism | Enable `PLAIN`/`LOGIN` at the relay, then build a new transport; it is `failed` |
| `PROTOCOL_REJECTED` | A protocol failure prevented a trustworthy outcome | Investigate the relay; a definitive SMTP refusal is a `rejected` result, not this error |
| `PROTOCOL_UNEXPECTED_RESPONSE` | Malformed or unexpected reply | Investigate; retry only if you know the stage |
| `PROTOCOL_CONNECTION_RESET` | Peer closed the connection | Retry only when you know it was before DATA; after DATA, expect `unknown` |

### Timeouts, lifecycle, and internal

| Code | Cause | Caller action |
| --- | --- | --- |
| `TIMEOUT_CONNECT` | Connect budget expired | Retryable when the network is healthy; check the relay |
| `TIMEOUT_OPERATION` | One reply took too long | Retry only if known pre-DATA |
| `TIMEOUT_TOTAL` | Whole-send budget expired | Retry only if known pre-DATA |
| `TIMEOUT_CLOSE` | Close drain exceeded its budget | Process-level concern; the transport is closed |
| `TRANSPORT_CLOSED` | Send after or during close | Create a new transport |
| `TRANSPORT_BUSY` | A send is already active on one native session | Internal to the engine, which queues instead; raise `delivery.maxQueue` if `LIMIT_QUEUE_FULL` appears |
| `LIMIT_QUEUE_FULL` | The configured queue bound was reached | Slow the producer, or raise `delivery.maxQueue` / `maxQueueBytes`; this is a local bound, not a provider limit |
| `INTERNAL` | Invariant violation or caught panic | Report a bug; do not retry |

## 3. Redaction

`details` may include `smtpCode`, `smtpEnhancedCode`, `host`, `port`,
`timeoutMs`, `elapsedMs`, `stage`, and `recipient`. It never includes passwords,
tokens, full bodies, attachment bytes, absolute file paths, or the username.
Error messages and logs follow the same rule; a redaction regression is treated
as a security failure in this repository.

Practical consequences:

- You can log `error.code` and `error.details` safely.
- Do not expect an absolute path back when a file attachment fails; you get
  `CONFIG_ATTACHMENT_UNREADABLE` and nothing that identifies the directory.
- Never send credentials into a custom `headers` value.

## 4. Security posture

- **TLS is mandatory**, with certificate and hostname verification against the
  bundled public roots. STARTTLS is supported; plaintext is not.
- **No verification escape hatch** exists in the API surface. Only
  `tlsRootCertificate` widens trust, and only additively (ADR-0018).
- **Credentials** are read from your environment or secret store and passed in
  `auth`. Sendust never logs them, and they do not appear in errors. AUTH
  mechanisms are `plain` and `login`.
- **Message content** is never logged. MIME serialization is deterministic and
  bounded before transmission.
- **Headers are screened** for CR/LF and control characters, and reserved names
  cannot be overridden.
- **Filesystem access is opt-in** and limited to bounded regular files; there is
  no URL, stream, or remote fetch path.
- **One SMTP transaction per session at a time**, with an in-process queue and a
  bounded pool of native sessions; no background worker and no persistent queue.
- **`accepted` is relay acceptance**, never mailbox delivery. There is no
  `delivered`, `opened`, `bounced`, or `queued` status.

## 5. Operations

### Relay outage

Connect and TLS failures surface as `PROTOCOL_*` or `TLS_*` errors, never as a
`rejected` result. A transient failure is retried inside the message's own send
budget; anything beyond one transport's lifetime is the caller's policy. The
queue is in-process only, so a restart loses whatever it still held.

### Ambiguous outcomes

`unknown` means the body was transmitted but the final disposition was not
observed. Treat it as **possibly delivered**:

- Do not resend automatically. If a resend is acceptable, do it deliberately and
  make duplication harmless — for example an application-generated idempotency
  key carried in `headers` plus a receiver-side dedupe rule.
- Log the correlated identifier, not the body.

### Shutdown

- `close()` in every shutdown path, and await it.
- Close while a send is in flight cancels or drains it within `closeMs`
  (125 s). A cancellation before the body reaches the wire is
  `TRANSPORT_CLOSED`; once the body may have been sent, the outcome is
  `unknown` and close does not rewrite it.
- There is no finalizer or process-exit hook; closing is the caller's job.

### Throttling under load

The engine queues concurrent sends and halves concurrency when a relay answers
with a `4xx` or a start attempt fails transiently, then grows back after
sustained success. `LIMIT_QUEUE_FULL` is a local bound: the queue reached
`delivery.maxQueue` or `maxQueueBytes`. Slow the producer or raise the bound; it
says nothing about the relay's own capacity.

### Deployment checklist

- [ ] Credentials come from the environment or a secret manager.
- [ ] Every transport is closed in `finally` and on shutdown.
- [ ] Timeouts are lowered where an interactive path cannot wait 120 s.
- [ ] `unknown` has an explicit application-level policy.
- [ ] `accepted` is not treated as delivery in logs or user messaging.
- [ ] Attachments stay within the caps, and path attachments only where the
      opt-in is intended.
- [ ] A target from the validated matrix is deployed; unsupported platforms fail
      at `createTransport` with `CONFIG_NATIVE_UNAVAILABLE`.

## 6. Related reading

- [Error model](../architecture/error-model.md) — the normative catalog.
- [Quick start](quick-start.md) — result semantics and `unknown`.
- [Lifecycle, timeouts, concurrency, and limits](lifecycle-limits.md) — budgets.
- [Addresses, MIME, and attachments](addressing-mime-attachments.md) — input rules.
