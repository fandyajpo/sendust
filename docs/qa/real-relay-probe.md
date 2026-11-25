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

## 3. The published 1.3.0 against Gmail

**Date:** 2026-11-25. **Build:** `@sendust/ts@1.3.0`, installed from the
registry into an empty folder, on Node.js 22.23.5.

| Field | Value |
| --- | --- |
| Relay | `smtp.gmail.com:465`, `tls`, app-password auth |
| Package | `@sendust/ts@1.3.0`, the pinned tarball, not a local build |
| Status | `accepted`, `250`, 4.8 s |
| One attempt | `delivery: { maxAttempts: 1 }`, so a transient reply would be reported, not retried away |

Gmail's own `EHLO`, read separately with `openssl s_client`, is what makes this
run exercise `1.3.0`'s new wire rather than the old one:

```text
250-smtp.gmail.com at your service, [38.86.221.124]
250-SIZE 35882577
250-8BITMIME
250-AUTH LOGIN PLAIN XOAUTH2 PLAIN-CLIENTTOKEN OAUTHBEARER XOAUTH
250-ENHANCEDSTATUSCODES
250-PIPELINING
250-CHUNKING
250-SMTPUTF8
250 LIMITS MAILMAX=1000 RCPTMAX=100
```

`SIZE` is advertised, so the core declared `SIZE=<n>` on `MAIL FROM` (RFC 1870)
and the relay accepted the message with that declaration — the first real relay to
see the `1.3.0` `SIZE` path. `XOAUTH2` is advertised too, so a token run on this
relay is meaningful; it has not been done, and it is the remaining real-relay gap
for the `AUTH` line change.

### A token far beyond the old 512-octet cap

A 1,472-character token proved the `AUTH` fix, and it needs no Google account:
the token was random bytes, not a credential, so nothing could leak. Gmail
answered:

| Field | Value |
| --- | --- |
| `error` | `PROTOCOL_UNEXPECTED_RESPONSE` |
| `stage` | `auth` |
| `smtpCode` | `555` |
| `elapsedMs` | 2.9 s |

A relay reply at `stage: auth` is the proof: the composed `AUTH XOAUTH2` line was
about 2,000 octets, and the relay read it and answered it. Under `1.2.1` the same
token never left the process, because the 512-octet command-line cap refused it
with `VALIDATION_HEADER_INJECTION` before a socket existed. This does not claim
that Gmail accepts XOAUTH2 with a valid token; it claims that the line reaches a
real relay, which is what the fix changed.

The same check on Node.js **21.2.0** failed before any socket with
`CONFIG_NATIVE_UNAVAILABLE`, "no require bridge on this runtime": 21 is below the
supported floor, so the loader refuses rather than loading something older than the
package supports. That is the documented floor working, not a defect.

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

