# Real-relay checks: Gmail

**Date:** 2026-11-20.
**Build:** `@sendust/ts@1.0.0` from the working tree (`ts/dist` plus the staged addon).
**Target:** `smtp.gmail.com`.

Two checks, in order: one with no credentials, which proves everything up to the
authentication requirement, and one authenticated send, which is the first real
message the project has delivered through a public relay.

## 1. Credential-free probe

This checks everything a real relay can prove without credentials: DNS, TCP, the
TLS handshake against the **public** trust store, EHLO and capability parsing,
the STARTTLS upgrade, and the `MAIL FROM` exchange. It does not send a message.

### Command

```text
node --input-type=module -e "
import { createTransport } from './ts/dist/index.js';
const probe = async (secure, port) => {
  const t = createTransport({ host: 'smtp.gmail.com', port, secure });
  try {
    const r = await t.send({ from: 'probe@example.org', to: ['probe@example.org'], subject: 'probe', text: 'probe' });
    console.log(secure, port, r.status, JSON.stringify(r.response));
  } catch (e) {
    console.log(secure, port, 'error', e.code, e.message);
  } finally { await t.close(); }
};
await probe('tls', 465);
await probe('starttls', 587);
"
```

No `tlsRootCertificate` is passed: the handshake uses the bundled WebPKI roots,
exactly as a consumer install does.

### Results

| Mode | Port | Outcome |
| --- | ---: | --- |
| `tls` (implicit) | 465 | `rejected`, `530 5.7.0 Authentication Required … gsmtp` |
| `starttls` | 587 | `rejected`, `530 5.7.0 Authentication Required … gsmtp` |

`rejected` is the correct outcome, not a failure: Gmail explicitly refused the
envelope because no credentials were offered. Reaching that reply proves the
connection, TLS verification, EHLO, capability parsing, STARTTLS upgrade, and
`MAIL FROM` all worked against the real server.

## 2. Authenticated send

`node examples/send_email.mjs` with the Gmail profile from `.env` (app password,
implicit TLS on 465):

| Field | Value |
| --- | --- |
| Relay | `smtp.gmail.com:465`, `tls`, auth `yes` |
| From | `Sendust Dev <n***@gmail.com>` |
| To | `f***@gmail.com` |
| Status | `accepted` |
| Recipient | `f***@gmail.com`: `accepted` |
| Relay reply | `250 2.0.0 OK 1795161309 … gsmtp` |

Addresses are redacted here because this repository is public. The send used the
real Gmail app password, read from `.env`, which is git-ignored and was never
written anywhere by the example.

This is the first authenticated real-relay send: AUTH, `MAIL FROM`, `RCPT TO`,
`DATA`, and the final `250` all worked, and the facade reported the relay's own
reply unchanged.

## What these checks do not prove

- **Not delivery.** `accepted` is the relay taking responsibility; nothing here
  says the mailbox received the message. Check the inbox separately.
- **Not a provider matrix.** Gmail only, one message, one account, one network.
- **Not a throughput claim.** These are single handshakes and one message; the
  comparison is [the benchmark](benchmark-nodemailer.md).

## Reproducing

1. `cp .env.example .env`, then set `SENDUST_USER`, `SENDUST_PASS` (app password,
   no spaces), `SENDUST_FROM`, and `SENDUST_TO`.
2. `npm run build:native && node examples/send_email.mjs`.

