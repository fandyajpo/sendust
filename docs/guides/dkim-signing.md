# DKIM signing

**Status:** user guide for the DKIM option added in `1.2.0`
([ADR-0024](../architecture/decisions/0024-dkim-signing.md)).
**Audience:** anyone who sends through an SMTP hop that does not sign for them.

Sendust signs with DKIM (RFC 6376) when you give it a key. It owns no DNS, keeps
no key store and rotates nothing: you hold the key, you publish the record, and
the library signs each message before the relay sees it.

## 1. What is signed

| Item | Value |
| --- | --- |
| Algorithm | `rsa-sha256` (RSASSA-PKCS1-v1_5) |
| Canonicalization | `relaxed/relaxed` |
| Signed headers | `from`, `to`, `cc`, `reply-to`, `subject`, `date`, `message-id`, `mime-version`, `content-type`, `content-transfer-encoding` |
| Body | hashed in full — no `l=` tag |
| Tags | `v`, `a`, `c`, `d`, `s`, `t`, `h`, `bh`, `b` |

- `From` is always signed, even if you pass a shorter `headers` list. A field the
  message does not carry is skipped rather than invented.
- `t=` is the same instant that `Date` and `Message-ID` carry, so the signature
  attests the headers the message actually has.
- `Bcc`, `Received`, `Return-Path`, `Comments`, `Keywords` and `DKIM-Signature`
  are refused in `headers` with `VALIDATION_INVALID_HEADER`: they are added or
  rewritten in transit, so signing one would make every signature fail.
- The signature is part of the serialized message, so `limits.maxWireBytes`
  covers it.

Not implemented: Ed25519 (`ed25519-sha256`), multiple signatures, `i=`/`q=`/`l=`
tags, and ARC.

## 2. Create the key

```text
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 -out dkim.pem
chmod 600 dkim.pem
```

- 2048 bits is the usual choice. Sendust accepts 1024 to 4096; 1024 is the RFC
  8301 floor and some receivers score it lower.
- Keep the file out of the repository and out of the image: read it from the
  environment or a secrets mount at start-up.
- One key per selector. Never reuse a selector for a different key.

## 3. Publish the record

The public key is the DER form of the SubjectPublicKeyInfo, base64 without line
breaks:

```text
openssl rsa -in dkim.pem -pubout -outform DER | base64 | tr -d '\n'
```

Publish it as one TXT record at `<selector>._domainkey.<domain>`:

```text
sendust._domainkey.example.test.  IN TXT  "v=DKIM1; k=rsa; p=MIICIjANBgkqhkiG9w0B..."
```

- `p=` is the value from the command above. A 2048-bit key makes it roughly 390
  characters, which is longer than one DNS string may be: split it into
  whitespace-separated 255-character quoted strings. Every provider's TXT editor
  does that for you when you paste the whole value into one record.
- `d=` is `example.test` and `s=` is `sendust` in this example, so the record must
  sit under `sendust._domainkey.example.test`.
- Check it before sending anything:

```text
dig +short TXT sendust._domainkey.example.test
opendkim-testkey -d example.test -s sendust
```

`opendkim-testkey` exits non-zero when the record is missing, the key does not
parse, or the selector has no matching DNS entry.

## 4. Turn it on

```js
import { createTransport } from "@sendust/ts";
import { readFileSync } from "node:fs";

const transport = createTransport({
  host: "smtp.example.test",
  port: 587,
  secure: "starttls",
  auth: { user: "sender@example.test", pass: process.env.SMTP_PASS },
  dkim: {
    domain: "example.test",
    selector: "sendust",
    privateKey: readFileSync("dkim.pem", "utf8"),
    // Optional: narrow the signed set. `from` is added back automatically.
    headers: ["from", "to", "subject", "date", "message-id"],
  },
});
```

- The key is parsed when the transport is created. A PEM that does not parse, or
  a key outside 1024–4096 bits, fails `createTransport` with
  `CONFIG_INVALID_DKIM_KEY` — before any connection, never after `DATA`.
- `domain` must be dot-separated ASCII labels and `selector` a single label. A
  wrong value is `VALIDATION_INVALID_OPTION`, and it would also break
  verification, because the receiver looks the record up under exactly those two
  names.
- Leaving `dkim` out sends an unsigned message, byte for byte as before.

## 5. Verify a real message

Send one message to a mailbox you control and read the headers:

- Gmail: **Show original**, then look for `dkim=pass header.d=example.test` in the
  `Authentication-Results` line.
- Any mailbox: the top of the raw message should start with
  `DKIM-Signature: v=1; a=rsa-sha256; c=relaxed/relaxed; d=example.test; s=sendust;`.
- Locally, with a saved raw message and [dkimpy](https://pypi.org/project/dkimpy/):

```text
python3 -c "import dkim; print(dkim.verify(open('message.eml','rb').read()))"
```

`True` means the signature and the published key agree. `False` almost always
means the DNS record does not match the key file, or that something between
Sendust and the mailbox rewrote a signed header or the body.

## 6. Rotate

1. Generate a new key and publish it under a **new** selector, for example
   `sendust2._domainkey`.
2. Wait for the record to resolve everywhere (`dig +short TXT` from a second
   network).
3. Change `selector` in the transport and redeploy.
4. Keep the old record for at least a week, because messages already in flight
   are verified against the selector they were signed with.

## 7. Troubleshooting

| Symptom | Cause | Fix |
| --- | --- | --- |
| `createTransport` throws `CONFIG_INVALID_DKIM_KEY` | Not a PEM private key, an encrypted key, or outside 1024–4096 bits | Export PKCS#1 or PKCS#8, unencrypted, and check the size |
| `VALIDATION_INVALID_HEADER` on a `headers` entry | The field is one RFC 6376 forbids signing, or is not a valid field name | Drop it from the list; `From` is always added |
| Signature present, receiver says `dkim=fail` (body hash) | A relay rewrote the body, or `p=` is a different key | Compare the key file with the published record |
| Signature present, receiver says `dkim=fail` (headers) | A relay rewrote `Subject`, `Date` or the address fields after signing | Sign fewer fields, or send directly to the final hop |
| No `DKIM-Signature` at all | `dkim` was not passed to this transport | Check the transport that actually sent the message |
| Send reports `accepted` but mail lands in spam | DKIM is only one of SPF, DKIM and DMARC | Publish SPF and a DMARC policy as well |

A signature that a receiver rejects is invisible to Sendust: relays accept by
envelope, so `accepted` keeps its meaning — the relay took responsibility — and
never becomes a claim about deliverability.

## 8. What this does not do

- No DNS lookup, no record creation, no key generation, no key storage.
- No ARC sealing, no DSN, no signature over `Received` chains.
- No per-message key choice: one signing identity per transport. To sign
  different domains, use one transport per identity.

See [ADR-0024](../architecture/decisions/0024-dkim-signing.md) for the decision,
[the public API contract](../architecture/public-api.md) for the option's
validation rules, and the [quick start](quick-start.md) for a first send.
