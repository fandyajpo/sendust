# Sendust — Error Model

**Status:** approved design baseline at P02-005; implementation mapping and golden tests remain P04/P05/P09 gates.  
**Related:** [system design](system-design.md), [ADR-0005 recipient semantics](decisions/0005-recipient-partial-failure.md), [security strategy](../engineering/security-strategy.md).

## 1. Two distinct channels

Sendust separates **outcomes** from **errors**.

- An **outcome** is a normal return value describing what the relay did: `accepted`, `rejected`, or `unknown`. It is not an exception.
- An **error** is a rejection describing why Sendust could not produce a trustworthy outcome: invalid input, TLS failure, authentication failure, timeout, closed transport, or internal fault.

A caller should branch on `result.status` for delivery semantics and on `error.code` for operational failures. Message strings are never a contract.

## 2. Result model

```ts
export type DeliveryStatus = "accepted" | "rejected" | "unknown";
export type RecipientStatus = "accepted" | "rejected" | "unknown";

export interface SendResult {
  status: DeliveryStatus;
  recipients: Array<{ address: string; status: RecipientStatus }>;
  response?: { code: number; message: string };
}
```

### 2.1 Delivery status rules

| Status | Meaning | When |
| --- | --- | --- |
| `accepted` | The relay returned a positive final response to DATA and accepted responsibility. **Not** mailbox delivery. | Final DATA response is 2xx. |
| `rejected` | The relay definitively refused the envelope or message. | A definitive negative SMTP response to MAIL, RCPT (under the selected recipient policy), DATA, or the final DATA response (4xx/5xx). |
| `unknown` | The message may or may not have been accepted; the final disposition cannot be established. | The DATA terminator may have reached the relay but the final response was not received, or a write/connection failure leaves acceptance genuinely uncertain. |

### 2.2 Recipient status rules

| Status | Meaning |
| --- | --- |
| `accepted` | The relay accepted this recipient (RCPT 2xx) **and** the message was accepted. |
| `rejected` | The relay explicitly refused this recipient at RCPT, or accepted its RCPT but explicitly refused the final DATA transaction. |
| `unknown` | The recipient was part of an envelope whose final status is unknown. |

**Critical rule (ADR-0005):** Sendust uses lettre's public lower-level async connection API to observe each RCPT reply. Explicitly refused recipients remain `rejected`; accepted recipients are sent one shared DATA transaction. RCPT acceptance alone never means message acceptance. A recipient becomes `accepted` only after positive final DATA. If final disposition is unknown, accepted-RCPT recipients become `unknown`. Sendust never sends one message per recipient.

### 2.3 No delivery claim

No field, name, or documentation may imply mailbox delivery. `accepted` is defined as relay acceptance. There is no `delivered`, `queued`, `opened`, or `bounced` status in the MVP.

### 2.4 No retry

`unknown` is terminal for the call. Sendust does not retry or resend. Sendust does not enqueue or schedule work. The caller decides whether a new send is safe.

## 3. Error taxonomy

```ts
export class SendustError extends Error {
  readonly code: string;
  readonly category: ErrorCategory;
  readonly details?: Record<string, unknown>;
  readonly cause?: unknown;
}

export type ErrorCategory =
  | "validation"
  | "configuration"
  | "limit"
  | "tls"
  | "authentication"
  | "protocol"
  | "timeout"
  | "closed"
  | "busy"
  | "internal";
```

### 3.1 Stable codes

| Code | Category | Public surface | Meaning | Retryable by caller? |
| --- | --- | --- | --- | --- |
| `VALIDATION_INVALID_OPTION` | validation | sync at createTransport; Promise rejection from send | Unknown/malformed transport option | No, fix input |
| `VALIDATION_INVALID_TIMEOUT` | validation | sync at createTransport | Non-finite, zero, negative, or out-of-range timeout | No |
| `VALIDATION_MISSING_FROM` | validation | Promise rejection from send | No From address | No |
| `VALIDATION_MISSING_RECIPIENT` | validation | Promise rejection from send | No To/CC/BCC | No |
| `VALIDATION_INVALID_ADDRESS` | validation | Promise rejection from send | Unparseable address | No |
| `VALIDATION_NON_ASCII_LOCAL_PART` | validation | Promise rejection from send | SMTPUTF8 not supported in MVP | No |
| `VALIDATION_INVALID_HEADER` | validation | Promise rejection from send | Illegal header name or value | No |
| `VALIDATION_HEADER_INJECTION` | validation | Promise rejection from send | CRLF/control characters in header or address | No |
| `VALIDATION_INVALID_ATTACHMENT` | validation | Promise rejection from send | Missing filename, invalid content type, unsupported shape | No |
| `CONFIG_UNSUPPORTED_SECURITY` | configuration | sync | Security mode not supported | No |
| `CONFIG_UNSUPPORTED_AUTH` | configuration | sync | Auth mode not supported | No |
| `CONFIG_FILE_ACCESS_DISABLED` | configuration | Promise rejection from send | File attachment used without opt-in | No |
| `CONFIG_INVALID_RELAY` | configuration | sync | Empty/invalid host or port | No |
| `LIMIT_RECIPIENTS` | limit | Promise rejection from send | More than 100 recipients | No |
| `LIMIT_ATTACHMENT_BYTES` | limit | Promise rejection from send | Attachment exceeds 10 MiB | No |
| `LIMIT_ATTACHMENT_TOTAL` | limit | Promise rejection from send | Raw attachments exceed 20 MiB | No |
| `LIMIT_WIRE_BYTES` | limit | Promise rejection from send | Serialized message exceeds 30 MiB | No |
| `TLS_CERTIFICATE` | tls | async | Certificate/hostname verification failed | No, configuration/trust issue |
| `TLS_HANDSHAKE` | tls | async | TLS negotiation failed | Maybe, only with corrected config |
| `TLS_PROTOCOL` | tls | async | Unsupported TLS version/cipher | No |
| `AUTH_FAILED` | authentication | async | Relay rejected credentials | No, fix credentials |
| `AUTH_UNSUPPORTED` | authentication | async | Relay offered no supported mechanism | No |
| `PROTOCOL_REJECTED` | protocol | async | Malformed or unexpected protocol failure that prevents a trustworthy outcome; definitive SMTP refusals are `rejected` outcomes | No |
| `PROTOCOL_UNEXPECTED_RESPONSE` | protocol | async | Malformed/unexpected reply | Maybe |
| `PROTOCOL_CONNECTION_RESET` | protocol | async | Peer closed unexpectedly | Only when known to be pre-DATA; if final disposition is uncertain after DATA, return `unknown` and do not automatically retry |
| `TIMEOUT_CONNECT` | timeout | async | Connect budget exceeded | Yes |
| `TIMEOUT_OPERATION` | timeout | async | Operation budget exceeded | Only when known to be pre-DATA; after DATA uncertainty, return `unknown` and do not automatically retry |
| `TIMEOUT_TOTAL` | timeout | async | Total send budget exceeded | Only when known to be pre-DATA; after DATA uncertainty, return `unknown` and do not automatically retry |
| `TIMEOUT_CLOSE` | timeout | async | Close drain exceeded budget | Process-level concern |
| `TRANSPORT_CLOSED` | closed | async | Send after close or during close | No |
| `TRANSPORT_BUSY` | busy | async | A send is already active | No, wait for completion |
| `INTERNAL` | internal | async | Invariant violation or unexpected panic | No, report bug |

Codes are append-only within a major version. Removing or renaming a code is a breaking change.

### 3.2 Error details

`details` may include:

- `smtpCode` (number) and `smtpEnhancedCode` (string) when the relay supplied them.
- `host` and `port`.
- `timeoutMs` and `elapsedMs`.
- `stage` only for coarse public-call boundaries documented by P01-003; never infer internal socket sub-stages.
- `recipient` for a validation error tied to one address.

`details` must never include passwords, full bodies, attachment bytes, tokens, or absolute paths.

## 4. Mapping from core and lettre

| Source | Core category | Public code |
| --- | --- | --- |
| Address parse/IDNA failure | validation | `VALIDATION_INVALID_ADDRESS` |
| Non-ASCII local part | validation | `VALIDATION_NON_ASCII_LOCAL_PART` |
| Header control characters | validation | `VALIDATION_HEADER_INJECTION` |
| Cap exceeded | limit | `LIMIT_*` |
| rustls certificate error | tls | `TLS_CERTIFICATE` |
| rustls handshake/protocol error | tls | `TLS_HANDSHAKE` / `TLS_PROTOCOL` |
| lettre auth error | authentication | `AUTH_FAILED` |
| TCP/connect/TLS/AUTH failure before an SMTP message transaction | operational error | `PROTOCOL_*`, `TLS_*`, or `AUTH_*`; never `rejected` delivery status |
| Explicit negative SMTP response at MAIL/RCPT/DATA or final DATA response | delivery outcome | `rejected` |
| Positive final DATA response | delivery outcome | `accepted` |
| Lost/missing final response after DATA terminator may have reached relay | delivery outcome | `unknown` |
| Timeout before any message acceptance is possible | timeout error | `TIMEOUT_*` |
| Timeout after DATA with uncertain final disposition | delivery outcome | `unknown`, with timeout metadata where available |
| State machine rejection | closed/busy | `TRANSPORT_CLOSED` / `TRANSPORT_BUSY` |
| Panic caught at boundary | internal | `INTERNAL` |

The mapping will be implemented in `crates/sendust-napi/src/error_map.rs` and tested with golden cases in P05-004/P09-003. This document does not claim those source files exist.

## 5. Error precedence

When multiple validation failures exist, the first failure in the deterministic order wins:

1. Transport configuration.
2. Message identity.
3. Recipient count and addresses.
4. Headers and CRLF.
5. Subject/body encoding.
6. Attachment metadata.
7. Attachment byte caps.
8. Wire cap.

This prevents nondeterministic messages and makes tests stable. The deterministic order is approved at P02-002 and implemented/tested in P04-001.

## 6. Redaction rules

| Data | In error message | In details | In logs |
| --- | --- | --- | --- |
| Password | Never | Never | Never |
| Auth token | Never | Never | Never |
| Full body | Never | Never | Never |
| Attachment bytes | Never | Never | Never |
| Attachment filename | Allowed (caller-supplied) | Allowed | Allowed |
| Absolute file path | Never | Never | Never; redact to basename or `<redacted>` |
| Host/port | Allowed | Allowed | Allowed |
| Username | Redacted/omitted | Omitted | Omitted |
| SMTP server text | Allowed if it contains no credentials | Allowed | Allowed |

Redaction is tested by serializing the error and scanning for marker values (P09-003, P11-004). A redaction regression is a security gate failure.

## 7. Panic policy

- Recoverable failures return `Result`; panics are reserved for violated internal invariants.
- `catch_unwind` is applied at exported napi boundaries where the panic strategy allows it.
- `panic = "abort"` is not used for the addon.
- A caught panic becomes `INTERNAL` with a generic message; the payload is not exposed.
- Panics on detached Tokio tasks must not leave a transport permanently `BUSY`; the state machine releases the slot on task completion or abort (tested in P10-001).

## 8. Consumer guidance (documented in P15-004)

- Branch on `result.status`, not on exception messages.
- Treat `unknown` as “do not automatically retry”; decide idempotency at the application layer.
- Treat `BUSY` as a programming error or a signal to serialize sends. Sendust has no internal queue, worker, scheduler, or queue adapter; any job orchestration belongs to the consuming application.
- Treat `accepted` as relay acceptance; delivery confirmation requires a separate mechanism not provided by Sendust.
