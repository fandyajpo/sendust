# Real-relay probe: Gmail, no credentials

**Date:** 2026-11-20.
**Build:** `@sendust/ts@1.0.0` from the working tree (`ts/dist` plus the staged addon).
**Target:** `smtp.gmail.com`, both security modes, no authentication.

This probe checks everything a real relay can prove without credentials: DNS,
TCP, the TLS handshake against the **public** trust store, EHLO and capability
parsing, the STARTTLS upgrade, and the `MAIL FROM` exchange. It does not send a
message, and it is not a delivery claim.

## Command

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

## Results

| Mode | Port | Outcome |
| --- | ---: | --- |
| `tls` (implicit) | 465 | `rejected`, `530 5.7.0 Authentication Required … gsmtp` |
| `starttls` | 587 | `rejected`, `530 5.7.0 Authentication Required … gsmtp` |

`rejected` is the correct outcome, not a failure: Gmail explicitly refused the
envelope because no credentials were offered. Reaching that reply proves the
connection, TLS verification, EHLO, capability parsing, STARTTLS upgrade, and
`MAIL FROM` all worked against the real server.

## What this does not prove

- **No authenticated send.** A real message needs an app password in `.env`;
  until that exists, no message has been delivered through a real relay.
- **No delivery.** As everywhere in Sendust, acceptance is relay acceptance.
- **No throughput claim.** This is one handshake per mode, not a benchmark.

## What an authenticated run adds

With `SENDUST_USER`, `SENDUST_PASS`, `SENDUST_FROM`, and `SENDUST_TO` set in
`.env`, `node examples/send_email.mjs` performs the same path plus AUTH and
`DATA`, and prints the relay's final reply. That is the only remaining
unverified step.
