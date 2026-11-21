# @sendust/ts

Standalone native email delivery engine for Node.js, Bun, and Deno. A narrow
TypeScript facade over a Sendust-owned Node-API addon.

Sendust submits one message per `send()` call through an SMTP relay you
configure. A result reports relay acceptance, never mailbox delivery.

## Status

Stable. `1.0.1` is published on npm and the `latest` tag resolves it. A
published version is never replaced, so this file reaches npm with the next
release.

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
  // Concurrent sends are queued, pooled, and scheduled automatically.
  const results = await Promise.all(
    recipients.map((to) =>
      transport.send({ from: "sender@example.test", to: [to], subject: "Hello", text: "Hello." }),
    ),
  );
  // Each result.status is "accepted", "rejected" or "unknown".
} finally {
  await transport.close();
}
```

## Guarantees

- TLS certificate and hostname verification are always on. There is no option
  to disable them and no plaintext mode.
- Concurrent `send()` calls are queued and run over a bounded pool of native
  sessions; one SMTP transaction runs per session at a time.
- Only transient failures are retried: a `4xx` final reply, or a connect or
  operation timeout before `DATA`. An `unknown` disposition is never retried.
- The queue is bounded; beyond it, `send()` rejects with `LIMIT_QUEUE_FULL`.
- Attachments from file paths are disabled unless `attachments.allowFileAccess`
  is set.

See the repository for the full public API contract and its ADRs.
