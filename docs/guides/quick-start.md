# Quick start and semantic contract

**Status:** user guide for the MVP; validated against PHASE-14 evidence.
**Audience:** application developers adding Sendust to a Node.js, Bun, or Deno service.

Sendust sends one message per `send()` call through an SMTP relay you configure.
The result describes what the **relay** did. It never claims that a mailbox
received anything.

## 1. What Sendust is, and is not

- A library you embed in your own application. It is not an SMTP server, a
  hosted service, a campaign tool, or a job scheduler.
- One transport owns one connection and runs **one send at a time**. There is no
  connection pool, no internal queue, and no automatic retry.
- TLS is mandatory. There is no plaintext mode and no way to disable certificate
  or hostname verification.
- You decide what to do about failures. Sendust reports them; it does not retry.

## 2. Installation

The npm package is pre-release and not published yet. Until publication, build
the package from a checkout and install the packed tarball:

```text
npm ci
npm run build:native            # stages ts/sendust.node
npm run build --workspace ts    # compiles dist/ (ESM and CommonJS)
npm run prepare:publish         # artifacts/publish/sendust-ts-0.1.0.tgz

npm install ./artifacts/publish/sendust-ts-0.1.0.tgz
```

The publish manifest declares the per-platform packages as
`optionalDependencies`, so a registry install would select the one matching your
OS and CPU. Those packages are not published either; for local use, install the
platform package built by the artifact jobs, or run from the repository.

Requirements:

- Node.js `>=22.23.5`, Bun 1.4.4, or Deno 2.9.8. Deno needs
  `--allow-ffi --allow-read` (add `--allow-net` if your code opens sockets).
- A supported OS/CPU pair: `linux-x64-gnu`, `linux-arm64-gnu`, `macos-arm64`,
  `macos-x64`, `windows-x64`. Everything else fails closed with
  `CONFIG_NATIVE_UNAVAILABLE` naming the package you would need.

The exact support evidence is in [platform validation](../qa/platform-validation.md)
and the [toolchain baseline](../engineering/toolchain-baseline.md).

## 3. One send

```js
import { createTransport, SendustError } from "@sendust/ts";

const transport = createTransport({
  host: "smtp.example.com",
  port: 465,
  secure: "tls",
  auth: {
    user: process.env.SMTP_USER,
    pass: process.env.SMTP_PASS,
  },
});

try {
  const result = await transport.send({
    from: "sender@example.com",
    to: ["recipient@example.com"],
    subject: "Hello",
    text: "Hello from Sendust.",
  });

  if (result.status === "accepted") {
    // The relay took responsibility for the message.
  } else if (result.status === "rejected") {
    // An explicit negative final reply; nothing was accepted. Do not retry
    // blindly — fix the cause (recipient, policy, content) first.
  } else {
    // "unknown": the final reply never arrived after the body was sent.
  }
} catch (error) {
  if (error instanceof SendustError) {
    // Stable code, category, and whitelisted details.
    console.error(error.code, error.message);
  }
  throw error;
} finally {
  await transport.close();
}
```

Rules that matter:

- Keep credentials out of source. Read them from the environment or a secret
  store.
- `close()` is required and belongs in `finally`. It is idempotent, bounded by
  `closeMs` (125 s by default), and never depends on garbage collection.
- `secure: "starttls"` is available for relays that require STARTTLS. There is
  no plaintext option.

## 4. What the result means

`send()` resolves for three outcomes and rejects with a typed error for
operational failures.

| Outcome | Meaning | Typical cause |
| --- | --- | --- |
| `accepted` | The relay accepted responsibility for the message. Not a delivery confirmation. | Final `DATA` reply was `2xx` |
| `rejected` | The relay explicitly refused the transaction. Nothing was accepted. | Final `DATA` reply was `4xx`/`5xx` |
| `unknown` | The body was transmitted, but the final reply was never observed. The relay may have accepted it. | Connection dropped, timeout, or close during `DATA` |

`result.recipients` records each `RCPT TO` outcome:

- A recipient is `accepted` only after the final `DATA` reply is `2xx`.
- A recipient refused at `RCPT` stays `rejected` even when the accepted subset
  succeeds in the same transaction.
- Recipients that were never attempted are `unknown`.

`result.response`, when present, carries the relay's final reply code and text
(for example `{ code: 451, message: "4.3.0 try again later" }`). It is absent for
`unknown`.

The full contract is [the public API document](../architecture/public-api.md).

### `unknown` is not a retry signal

The library never resends. Whether to retry an `unknown` outcome is your
decision, and only with caller-level idempotency: the same message may already
be in the queue of the relay. A safe pattern is a caller-generated idempotency
key in `headers` plus a supplier-side dedupe rule, or accepting the risk of a
duplicate deliberately.

### BUSY

One transport runs one send. A second `send()` while the first is in flight
rejects immediately with `TRANSPORT_BUSY` and opens no connection. Do not build
an internal queue around it; create another transport when you need concurrency,
and accept that each owns its own connection.

## 5. Try it without a relay

This is the shape of the executable check in
[`tests/docs/quickstart.mjs`](../../tests/docs/quickstart.mjs): a loopback TLS
relay with a test certificate, trusted through the additive
`tlsRootCertificate`.

```js
import { readFileSync } from "node:fs";
import { createTransport } from "@sendust/ts";

// `relay` below is the loopback TLS server from the executable check: it greets,
// answers EHLO/MAIL/RCPT, accepts DATA and replies `250`.

const transport = createTransport({
  host: "127.0.0.1",
  port: relay.port,
  secure: "tls",
  // Test only. Production trusts the public roots and omits this option.
  tlsRootCertificate: readFileSync("tests/fixtures/tls/ca.crt", "utf8"),
});

try {
  const result = await transport.send({
    from: "sender@example.test",
    to: ["recipient@example.test"],
    subject: "Hello",
    text: "Hello from Sendust.",
  });
  console.log(result.status); // "accepted"
} finally {
  await transport.close();
}
```

`tlsRootCertificate` adds anchors to the bundled public roots. It cannot remove
roots, cannot disable verification, and a malformed PEM fails with
`VALIDATION_INVALID_OPTION`.

## 6. Where to go next

- Lifecycle, timeouts, concurrency, and hard caps: P15-002 guide.
- Addresses, MIME, and attachments: P15-003 guide.
- Error catalog, security, and operations: P15-004 guide.
- Maintainer guide and testing map: P15-005 guide.
