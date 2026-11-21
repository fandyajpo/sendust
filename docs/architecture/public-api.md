# Sendust — Public API Contract

**Status:** architecture baseline approved at P02-005; public API implementation and contract gate remain P09-001–P09-005.  
**Types are illustrative:** the authoritative declarations are generated/implemented in P09-001; this document defines behavior, not final syntax.

## 1. Surface

```ts
import { createTransport } from "sendust";
```

The MVP exports exactly:

- `createTransport(options): Transport`
- `Transport` interface
- `SendResult`, `Message`, `Attachment`, and option types
- Error classes and `ErrorCategory`
- `version` (package version string)

No plugin registry, no transport subclasses, no streaming API, no callback API, no template helper.

## 2. Transport options

| Option | Type | Default | Validation |
| --- | --- | --- | --- |
| `host` | `string` | required | non-empty, no whitespace/control chars |
| `port` | `number` | required | integer 1–65535 |
| `secure` | `"tls" \| "starttls"` | required | one of the two; no plaintext mode |
| `auth` | `{ user, pass, method? }` | undefined | user non-empty; `method` `"plain"` or `"login"` |
| `ehloName` | `string` | `localhost` | one printable ASCII token: 1–255 bytes, no space, control or non-ASCII character |
| `tlsRootCertificate` | `string` | none | one PEM bundle of trust anchors added on top of the bundled roots; never replaces them and never disables verification; a malformed PEM is `VALIDATION_INVALID_OPTION` (ADR-0018) |
| `timeouts.connectMs` | `number` | 10000 | integer 1000–10000; ≤ totalMs |
| `timeouts.operationMs` | `number` | 30000 | integer 1000–30000; ≤ totalMs |
| `timeouts.totalMs` | `number` | 120000 | integer 1000–120000; ≥ connectMs and operationMs |
| `timeouts.closeMs` | `number` | 125000 | integer totalMs–125000; ≥ totalMs |
| `attachments.allowFileAccess` | `boolean` | `false` | boolean |
| `limits.maxRecipients` | `number` | 100 | integer, 1–100 (cannot exceed hard cap) |
| `limits.maxAttachmentBytes` | `number` | 10485760 | integer, 1–10485760 |
| `limits.maxTotalAttachmentBytes` | `number` | 20971520 | integer, 1–20971520 |
| `limits.maxWireBytes` | `number` | 31457280 | integer, 1–31457280 |

Timeout values may be lowered but not raised above the documented maxima. Limits may be lowered but not raised. A caller cannot configure a value above the documented hard cap. Unknown option keys are rejected with `VALIDATION_INVALID_OPTION` to catch typos.

There is no option to disable TLS verification, no plaintext SMTP mode, no pool size, no retry count, and no proxy configuration in MVP. The only trust input is the additive `tlsRootCertificate` (ADR-0018).

## 3. Message shape

| Field | Type | Required | Rules |
| --- | --- | --- | --- |
| `from` | string or `{ name?, address }` | yes | ASCII local part; IDNA domain; optional Unicode display name |
| `to` | array | no; at least one recipient across `to`, `cc`, and `bcc` is required | same address rules |
| `cc` | array | no | same |
| `bcc` | array | no | same; never written to headers |
| `replyTo` | array | no | same |
| `subject` | string | yes | non-empty and not whitespace-only; Unicode allowed; no control characters |
| `text` | string | one of text/html | Unicode allowed |
| `html` | string | one of text/html | Unicode allowed |
| `headers` | `Record<string,string>` | no | valid field names; reserved names rejected |
| `attachments` | array | no | see below |

Reserved header names that cannot be overridden: `From`, `To`, `Cc`, `Bcc`, `Reply-To`, `Subject`, `Date`, `Message-ID`, `MIME-Version`, `Content-Type`, `Content-Transfer-Encoding`. Attempting to set them via `headers` fails validation. Sendust sets `Date` (UTC) and a `Message-ID` itself for every send.

If both `text` and `html` are present, the message is `multipart/alternative`; if neither is present, validation fails. Empty string bodies are allowed and produce an empty part.

## 4. Attachments

```ts
type Attachment =
  | { filename: string; content: Buffer | Uint8Array; contentType?: string }
  | { path: string; filename?: string; contentType?: string };
```

- Buffer attachments are always allowed within caps.
- Path attachments require `attachments.allowFileAccess: true`; otherwise `CONFIG_FILE_ACCESS_DISABLED`.
- Path attachments read only regular files; symlinks, directories, FIFOs, devices, and sockets are rejected.
- `contentType` must be a syntactically valid MIME type if provided; otherwise it is inferred from the filename extension using a fixed, documented map, falling back to `application/octet-stream`. The map is: `txt` → `text/plain`; `html`/`htm` → `text/html`; `csv` → `text/csv`; `md` → `text/markdown`; `ics` → `text/calendar`; `json` → `application/json`; `pdf` → `application/pdf`; `zip` → `application/zip`; `png` → `image/png`; `jpg`/`jpeg` → `image/jpeg`; `gif` → `image/gif`; `webp` → `image/webp`; `svg` → `image/svg+xml`; anything else → `application/octet-stream`. Matching is ASCII case-insensitive.
- Filenames are encoded per RFC 2231/2047 rules; CRLF and control characters are rejected.
- No `url`, `href`, `stream`, or remote attachment form exists.

## 5. Send

```ts
const transport = createTransport({ host, port, secure: "tls", auth });
try {
  const result = await transport.send({ from, to, subject, text });
  if (result.status === "accepted") {
    // relay accepted responsibility; not a delivery confirmation
  }
} finally {
  await transport.close();
}
```

Behavior:

- Resolves with `SendResult` for `accepted`, `rejected`, and `unknown`.
- `SendResult.recipients` records each explicit RCPT outcome. A recipient is `accepted` only after final DATA 2xx; RCPT refusals remain `rejected` even when the accepted subset succeeds.
- Mixed RCPT results produce one DATA transaction for the accepted subset, never one message per recipient.
- A missing/indeterminate final DATA disposition after message transmission begins produces `unknown` for the RCPT-accepted subset.
- Rejects with a `SendustError` subclass for validation, configuration, limit, TLS, auth, timeout, closed, busy, and internal failures. The classes are `SendustValidationError`, `SendustConfigurationError`, `SendustLimitError`, `SendustTlsError`, `SendustAuthenticationError`, `SendustProtocolError`, `SendustTimeoutError`, `SendustClosedError`, `SendustBusyError`, and `SendustInternalError`; all extend `SendustError` and carry the stable `code`, `category`, and whitelisted `details`. A missing or unloadable bundled native addon rejects synchronously at `createTransport` with `CONFIG_NATIVE_UNAVAILABLE`, and no fallback build is used.
- Never resolves with a delivery claim.
- Never retries.
- Never queues.
- Never sends per-recipient messages.

The result shape is:

```ts
export type DeliveryStatus = "accepted" | "rejected" | "unknown";
export type RecipientStatus = "accepted" | "rejected" | "unknown";

export interface SendResult {
  status: DeliveryStatus;
  recipients: Array<{ address: string; status: RecipientStatus }>;
  response?: { code: number; message: string };
}
```

## 6. Close

```ts
await transport.close();
```

- Idempotent; repeated calls resolve once the transport is closed.
- Bounded by `closeMs` (125 s default).
- Transitions `open -> closing -> closed`.
- Close requests cancellation of an in-flight operation and awaits bounded cleanup. If cancellation occurs once message transmission has begun and the final SMTP disposition is not observed, the send outcome is `unknown`; close must not rewrite that uncertainty as a definitive cancellation or rejection.
- After close, `send` rejects with `TRANSPORT_CLOSED`.
- No GC/finalizer dependency.

## 7. State

```ts
transport.state; // "open" | "closing" | "closed"
```

Read-only. Reflects the state machine synchronously.

## 8. Concurrency and delivery management

- `send()` is queued: concurrent calls are accepted and scheduled, not rejected.
- The engine runs up to `delivery.maxConcurrent` sends at once (8 by default)
  over at most `delivery.maxConnections` native sessions, and each session runs
  one SMTP transaction at a time.
- Concurrency rises to the work already waiting, up to `delivery.maxConcurrent`;
  a `4xx` final reply or a transient start failure halves it and starts a
  cooldown, so it cannot oscillate.
- Only transient failures are retried: a `4xx` final reply, `TIMEOUT_CONNECT`,
  `TIMEOUT_OPERATION`, `TIMEOUT_TOTAL`, and `PROTOCOL_CONNECTION_RESET`.
  `unknown` is never retried, and `AUTH_*` stops the transport.
- The queue is bounded by `delivery.maxQueue` and `delivery.maxQueueBytes`;
  beyond either, `send()` rejects with `LIMIT_QUEUE_FULL` instead of exhausting
  memory.
- A session is reused across sequential messages: after a definitive outcome it
  is reset with `RSET` and kept, up to `delivery.maxMessagesPerSession` messages
  or `delivery.sessionIdleMs` of inactivity. An error or an `unknown`
  disposition drops it ([ADR-0022](decisions/0022-session-reuse.md)).
- `transport.stats` reports queue depth, active sends, pool size, concurrency,
  and outcome counters.
- Automatic management is the default; every option is an override
  ([ADR-0021](decisions/0021-autonomous-delivery-engine.md)).

## 9. Module formats

| Consumer | Entry |
| --- | --- |
| ESM | `import { createTransport } from "sendust"` |
| CommonJS | `const { createTransport } = require("sendust")` |
| TypeScript | types resolved through `exports.types` |

Both formats must expose identical classes and behavior. A class thrown from the native layer must satisfy `instanceof` against the wrapper-exported class in both formats (tested in P14-004).

## 10. Non-goals in the API

The following are intentionally absent and must not appear in the MVP type surface:

- `pool`, `maxConnections`, `maxMessages`
- `retry`, `retries`, `backoff`
- `queue`, `worker`, `scheduler`, `job`, `queueAdapter`
- `abortSignal` option; the MVP only exposes explicit bounded `close()` and timeout-based internal cancellation
- `stream`, `streamTransport`
- `url` attachments
- `dkim`
- `template`, `render`
- provider-specific options (`ses`, `sendgrid`, `mailgun`)
- `verify()` connection probe (post-MVP candidate)
- `sendMail` callback form

## 11. Compatibility rules

- Additive optional fields are minor versions.
- New error codes are minor versions.
- Changing a status meaning, removing a code, or changing a default is a major version.
- Tightening a limit is a major version; loosening is minor.
- Node.js, Bun, Deno, and platform support claims follow the exact tested matrix in [the toolchain baseline](../engineering/toolchain-baseline.md) and are documented per release.
