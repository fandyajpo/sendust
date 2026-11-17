# @sendust/ts

Standalone native email delivery engine for Node.js, Bun, and Deno. A narrow
TypeScript facade over a Sendust-owned Node-API addon.

Sendust submits one message per `send()` call through an SMTP relay you
configure. A result reports relay acceptance, never mailbox delivery.

## Status

Pre-release. The package is not published, and no runtime is supported until
the exact runtime and platform evidence in the Sendust repository passes.

## Usage

```js
import { createTransport } from "@sendust/ts";

const transport = createTransport({
  host: "smtp.example.test",
  port: 465,
  secure: "tls",
  auth: { user: "sender@example.test", pass: process.env.SMTP_PASS },
});

try {
  const result = await transport.send({
    from: "sender@example.test",
    to: ["recipient@example.test"],
    subject: "Hello",
    text: "Hello from Sendust.",
  });
  // result.status is "accepted", "rejected" or "unknown".
} finally {
  await transport.close();
}
```

## Guarantees

- TLS certificate and hostname verification are always on. There is no option
  to disable them and no plaintext mode.
- A transport runs one send at a time. An overlapping `send()` rejects with
  `TRANSPORT_BUSY` and opens no connection.
- There is no automatic retry, connection pool, or internal queue.
- Attachments from file paths are disabled unless `attachments.allowFileAccess`
  is set.

See the repository for the full public API contract and its ADRs.
