# Sendust — Public API Contract

**Status:** proposed; frozen at `PHASE-02`, implemented in `PHASE-09`.  
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
| `ehloName` | `string` | system hostname or `localhost` | valid hostname token; no CRLF |
| `timeouts.connectMs` | `number` | 10000 | finite, > 0, ≤ total |
| `timeouts.operationMs` | `number` | 30000 | finite, > 0 |
| `timeouts.totalMs` | `number` | 120000 | finite, > 0, ≥ connect |
| `timeouts.closeMs` | `number` | 125000 | finite, > 0 |
| `attachments.allowFileAccess` | `boolean` | `false` | boolean |
| `limits.maxRecipients` | `number` | 100 | integer, 1–100 (cannot exceed hard cap) |
| `limits.maxAttachmentBytes` | `number` | 10485760 | integer, 1–10485760 |
| `limits.maxTotalAttachmentBytes` | `number` | 20971520 | integer, 1–20971520 |
| `limits.maxWireBytes` | `number` | 31457280 | integer, 1–31457280 |

Limits may be lowered but not raised. A caller cannot configure a value above the documented hard cap. Unknown option keys are rejected with `VALIDATION_INVALID_OPTION` to catch typos.

There is no option to disable TLS verification, no plaintext SMTP mode, no pool size, no retry count, and no proxy configuration in MVP.

## 3. Message shape

| Field | Type | Required | Rules |
| --- | --- | --- | --- |
| `from` | string or `{ name?, address }` | yes | ASCII local part; IDNA domain; optional Unicode display name |
| `to` | array | no; at least one recipient across `to`, `cc`, and `bcc` is required | same address rules |
| `cc` | array | no | same |
| `bcc` | array | no | same; never written to headers |
| `replyTo` | array | no | same |
| `subject` | string | yes | may be empty? No: non-empty, Unicode allowed, no CRLF |
| `text` | string | one of text/html | Unicode allowed |
| `html` | string | one of text/html | Unicode allowed |
| `headers` | `Record<string,string>` | no | valid field names; reserved names rejected |
| `attachments` | array | no | see below |

Reserved header names that cannot be overridden: `From`, `To`, `Cc`, `Bcc`, `Reply-To`, `Subject`, `Date`, `Message-ID`, `MIME-Version`, `Content-Type`, `Content-Transfer-Encoding`. Attempting to set them via `headers` fails validation.

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
- `contentType` must be a syntactically valid MIME type if provided; otherwise it is inferred from the filename extension using a fixed, documented map, falling back to `application/octet-stream`.
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
- Rejects with a `SendustError` subclass for validation, configuration, limit, TLS, auth, timeout, closed, busy, and internal failures.
- Never resolves with a delivery claim.
- Never retries.
- Never queues.
- Never sends per-recipient messages.

## 6. Close

```ts
await transport.close();
```

- Idempotent; repeated calls resolve once the transport is closed.
- Bounded by `closeMs` (125 s default).
- Transitions `open -> closing -> closed`.
- Close requests cancellation of an in-flight operation and awaits bounded cleanup. If cancellation occurs after message data may have reached the relay but before a final SMTP reply is observed, the send outcome is `unknown`; close must not rewrite that uncertainty as a definitive cancellation or rejection.
- After close, `send` rejects with `TRANSPORT_CLOSED`.
- No GC/finalizer dependency.

## 7. State

```ts
transport.state; // "open" | "closing" | "closed"
```

Read-only. Reflects the state machine synchronously.

## 8. Concurrency

- One active send per transport.
- Concurrent `send` rejects immediately with `TRANSPORT_BUSY` and performs no I/O.
- To send concurrently, create multiple transports; each owns its own connection. The MVP does not pool.

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
- `abortSignal` as a required option
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
- Node.js and platform support changes follow the published matrix and are documented per release.
