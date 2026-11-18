# Addresses, MIME, and attachments

**Status:** user guide for the MVP; matched to `crates/sendust-core/src/address`,
`message`, `mime`, and `attachments`, and to the public contract in
[the public API document](../architecture/public-api.md).

## 1. Addresses

`from`, `to`, `cc`, `bcc`, and `replyTo` accept either a plain string or
`{ name?, address }`.

```js
const message = {
  from: { name: "Sendust", address: "sender@example.com" },
  to: ["recipient@example.com", { name: "Ops", address: "ops@example.com" }],
  cc: ["audit@example.com"],
  subject: "Hello",
  text: "Body",
};
```

Rules:

- **Local parts are ASCII.** `üser@example.com` is rejected with
  `VALIDATION_NON_ASCII_LOCAL_PART`. SMTPUTF8 is post-MVP, so non-ASCII local
  parts are never sent.
- **Domains are IDNA-processed.** `mail@bücher.example` becomes the punycode
  domain on the wire; the caller writes the Unicode form.
- **Display names may be Unicode** and are encoded in headers.
- **CR, LF, and control characters are rejected** in any address or display
  name with `VALIDATION_HEADER_INJECTION`.
- **A missing `from`** fails with `VALIDATION_MISSING_FROM`.
- **At least one recipient** across `to`, `cc`, and `bcc` is required, otherwise
  `VALIDATION_MISSING_RECIPIENT`.
- **An unparseable address** fails with `VALIDATION_INVALID_ADDRESS`.
- `bcc` recipients receive the message but never appear in headers.

## 2. Subject and headers

- `subject` is required, must not be empty or whitespace-only, may be Unicode,
  and must not contain control characters.
- `headers` is a `Record<string, string>` or an array of `{ name, value }`.
- **Reserved names are rejected** with `VALIDATION_INVALID_HEADER`: `From`, `To`,
  `Cc`, `Bcc`, `Reply-To`, `Subject`, `Date`, `Message-ID`, `MIME-Version`,
  `Content-Type`, `Content-Transfer-Encoding`.
- Sendust sets `Date` (UTC) and a fresh `Message-ID` for every send.
- A header name that is not a valid field name, or a value with CR/LF or
  control characters, fails with `VALIDATION_INVALID_HEADER` or
  `VALIDATION_HEADER_INJECTION`.
- Long or non-ASCII header values are folded and encoded per the MIME rules the
  serializer applies; callers never fold by hand.
- Unknown transport or message keys are rejected (`VALIDATION_INVALID_OPTION`)
  so a typo cannot be silently ignored.

## 3. Bodies and MIME shape

Provide `text`, `html`, or both:

| Input | Rendered as |
| --- | --- |
| `text` only | `text/plain` |
| `html` only | `text/html` |
| both | `multipart/alternative`, `text/plain` first |
| attachments present | wrapped in `multipart/mixed` around the body part(s) |

- Empty strings are allowed and produce an empty part.
- Neither body present fails validation.
- Unicode bodies are transfer-encoded as needed; the serialized length is what
  the wire cap measures (30 MiB, exact and inclusive).

## 4. Attachments

```ts
type Attachment =
  | { filename: string; content: Buffer | Uint8Array; contentType?: string }
  | { path: string; filename?: string; contentType?: string };
```

### Buffer attachments

Always allowed within the caps. The bytes are copied synchronously when
`send()` is called, so mutating or detaching the source afterwards does not
affect the message.

### Path attachments require opt-in

Filesystem access is **disabled by default**:

```js
const transport = createTransport({
  host: "smtp.example.com",
  port: 465,
  secure: "tls",
  attachments: { allowFileAccess: true },
});
```

Without the opt-in, a path attachment fails with
`CONFIG_FILE_ACCESS_DISABLED`.

With the opt-in:

- Only **bounded regular files** are read. A missing file, a directory, a
  symlink, a FIFO, a device, or an unreadable path fails with
  `CONFIG_ATTACHMENT_UNREADABLE` — nothing blocks on a FIFO or device.
- The per-file cap is enforced while reading (`LIMIT_ATTACHMENT_BYTES`), and the
  aggregate cap across all attachments (`LIMIT_ATTACHMENT_TOTAL`). Reading stops
  at the cap; a file that grows during the read cannot exceed it.
- There is **no URL, stream, or remote fetch** form. Sendust never fetches an
  attachment over the network.

### Filenames and content types

- A filename containing a path is reduced to its display name; the directory
  part is never used.
- Empty or whitespace-only filenames are rejected with
  `VALIDATION_INVALID_ATTACHMENT`.
- Filenames with CR/LF or control characters are rejected, and non-ASCII names
  are encoded per RFC 2231.
- `contentType` must be a valid MIME type when given. Otherwise it is inferred
  from the extension (case-insensitive), falling back to
  `application/octet-stream`.

| Extension | Inferred type |
| --- | --- |
| `txt` | `text/plain` |
| `html`, `htm` | `text/html` |
| `csv` | `text/csv` |
| `md` | `text/markdown` |
| `ics` | `text/calendar` |
| `json` | `application/json` |
| `pdf` | `application/pdf` |
| `zip` | `application/zip` |
| `png` | `image/png` |
| `jpg`, `jpeg` | `image/jpeg` |
| `gif` | `image/gif` |
| `webp` | `image/webp` |
| `svg` | `image/svg+xml` |
| anything else | `application/octet-stream` |

## 5. Accepted and rejected examples

| Input | Outcome |
| --- | --- |
| `to: ["a@example.com"]`, `text: "hi"` | Accepted |
| `{ name: "Zoë", address: "z@example.com" }` as `from` | Accepted; display name encoded |
| `to: ["üser@example.com"]` | `VALIDATION_NON_ASCII_LOCAL_PART` |
| `to: []` with no `cc`/`bcc` | `VALIDATION_MISSING_RECIPIENT` |
| `subject: "  "` | `VALIDATION_INVALID_OPTION` |
| `headers: { Subject: "x" }` | `VALIDATION_INVALID_HEADER` |
| `headers: { "X-Note": "a\r\nBcc: b@example.com" }` | `VALIDATION_HEADER_INJECTION` |
| `attachments: [{ path: "/tmp/a.txt", filename: "a.txt" }]` without opt-in | `CONFIG_FILE_ACCESS_DISABLED` |
| `attachments: [{ path: "/tmp/missing.txt" }]` with opt-in | `CONFIG_ATTACHMENT_UNREADABLE` |
| 12 MiB buffer with the default cap | `LIMIT_ATTACHMENT_BYTES` |
| `{ url: "https://example.com/a.pdf" }` | `VALIDATION_INVALID_ATTACHMENT` (no such shape) |

The complete list, with retryability and redaction notes, is in
[the error model](../architecture/error-model.md).

## 6. Related reading

- [Quick start](quick-start.md) — the first send and result semantics.
- [Lifecycle, timeouts, concurrency, and limits](lifecycle-limits.md) — cap
  boundary behaviour.
