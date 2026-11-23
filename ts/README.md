# @sendust/ts

Native SMTP for Node.js, Bun and Deno: a Rust core behind a narrow TypeScript
API. One message per `send()` call, through the relay you configure.

A result reports what the relay said — `accepted`, `rejected` or `unknown` — and
never claims that a mailbox received anything.

## Install

```sh
npm install @sendust/ts
```

`pnpm add @sendust/ts`, `yarn add @sendust/ts` and `bun add @sendust/ts` work the
same way. There is no postinstall, no compiler and no download: the addon for your
platform arrives as an optional dependency of the same version.

## Quick start

```js
import { createTransport } from "@sendust/ts";

const transport = createTransport({
  host: "smtp.example.test",
  port: 587,
  secure: "starttls", // or "tls" for implicit TLS, usually port 465
  auth: { user: "sender@example.test", pass: process.env.SMTP_PASS },
});

try {
  // Concurrent sends are queued, pooled and scheduled automatically.
  const results = await Promise.all(
    recipients.map((to) =>
      transport.send({
        from: "sender@example.test",
        to: [to],
        subject: "Hello",
        text: "Hello.",
      }),
    ),
  );
  for (const result of results) {
    console.log(result.status); // "accepted" | "rejected" | "unknown"
  }
} finally {
  await transport.close();
}
```

## Authentication

```js
// Password: AUTH PLAIN or LOGIN, chosen from what the relay advertises.
auth: { user: "sender@example.test", pass: process.env.SMTP_PASS }

// OAuth 2.0: AUTH XOAUTH2 with a bearer token, never chosen implicitly.
// Pass a function to hand each new session a fresh token.
auth: { user: "sender@example.test", accessToken: () => getToken(), method: "xoauth2" }
```

## DKIM signing

Optional, and signed on the composed bytes, so what the socket receives is what
the signature covers.

```js
import { readFileSync } from "node:fs";

const transport = createTransport({
  host: "smtp.example.test",
  port: 587,
  secure: "starttls",
  auth: { user: "sender@example.test", pass: process.env.SMTP_PASS },
  dkim: {
    domain: "example.test",
    selector: "sendust",
    privateKey: readFileSync("dkim.pem", "utf8"), // PEM, 1024-4096 bits
  },
});
```

`rsa-sha256` with relaxed/relaxed canonicalization, the body hashed in full (no
`l=` tag), and `From` always signed. The key is parsed when the transport is
created, so an unusable one fails with `CONFIG_INVALID_DKIM_KEY` before any
connection. Sendust publishes no DNS record and stores no key: the selector and
its `_domainkey` record are yours.

## What the engine does

- **Queue and pool.** Concurrent `send()` calls are scheduled over a bounded pool
  of native sessions, one SMTP transaction per session at a time.
- **Sessions stay warm.** A definitive outcome keeps the authenticated session for
  the next message, up to `delivery.maxMessagesPerSession` or
  `delivery.sessionIdleMs`; an error or `unknown` drops it.
- **Retries only where they are safe.** A `4xx` final reply, or a connect or
  operation timeout before `DATA`, is retried with backoff. An `unknown`
  disposition never is, because a retry after an uncertain `DATA` can deliver
  twice.
- **Hard caps.** 100 recipients, 10 MiB per attachment, 20 MiB of attachments,
  30 MiB of serialized message, and four timeout budgets — all lowerable, never
  raisable.
- **TLS cannot be weakened.** Certificate and hostname verification are always on,
  there is no plaintext mode, and the only trust input adds anchors.
- **File access is opt-in.** Attachments from paths need
  `attachments.allowFileAccess: true`; bytes in memory need no opt-in.

## Platforms

The addon ships as one optional dependency per target, all at the same version:
`linux-x64-gnu`, `linux-arm64-gnu`, `linux-x64-musl`, `linux-arm64-musl`,
`macos-arm64`, `macos-x64`, `windows-x64`. npm installs only the one that matches.
A platform without a build fails closed at `createTransport` with
`CONFIG_NATIVE_UNAVAILABLE`, naming the package it needed — never a raw
`MODULE_NOT_FOUND`.

## Requirements

- Node.js `>=22.23.5`, Bun `1.4.4`, or Deno `2.9.8`.
- Deno needs `--allow-ffi --allow-read`, plus `--allow-net` if your code opens the
  sockets itself.

## Documentation

- The documentation site: <https://sendust-docs.vercel.app>
- In the repository: [quick start](https://github.com/fandyajpo/sendust/blob/main/docs/guides/quick-start.md),
  [lifecycle and limits](https://github.com/fandyajpo/sendust/blob/main/docs/guides/lifecycle-limits.md),
  [errors and operations](https://github.com/fandyajpo/sendust/blob/main/docs/guides/errors-security-operations.md),
  [DKIM signing](https://github.com/fandyajpo/sendust/blob/main/docs/guides/dkim-signing.md).

## Licence

MIT or Apache-2.0, at your option.
