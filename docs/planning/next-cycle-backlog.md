# Next-cycle backlog (after 1.0.2)

**Date:** 2026-11-21.
**Baseline:** `@sendust/ts@1.0.2`, published and tagged
([release gate](../gates/release-1.0.2.md), [release notes](../releases/1.0.2.md)).
**Supersedes the ordering in:** [post-MVP backlog](post-mvp-backlog.md), which was
written against `0.1.0-rc.1` and before the delivery engine existed.
**What this is:** a prioritized shortlist with the ADR, the evidence, and the
entry criteria each item needs. It authorizes nothing; each row starts with its
own ADR and release gate.

The ranking is adoption-first, from the
[competitive landscape](../research/competitive-landscape.md): what stops a real
team from installing Sendust today comes before what would make it nicer to use.

## 1. Priority 0 — close the loops that are open now

| Item | Why | Size | Version |
| --- | --- | --- | --- |
| Clean-install verification of `1.0.2` on each claimed platform | The publication record verified the tarball hash and dist-tags but not a fresh install ([checklist](../engineering/publication-checklist.md) §5c) | Small | — |
| Move to npm trusted publishing (OIDC) with provenance | Bypass-2FA direct publishing is deprecated in January 2027, and Nodemailer and Resend already publish with SLSA attestations while `@sendust/ts` publishes with a token and no attestation | Small (owner configures npm, workflow change) | — |
| Re-run the owner's Gmail benchmark against `1.0.x` | The only real-relay comparison on record is `1.0.0`; the `1.0.1` tail fix is still a projection | Small (harness exists) | — |

None of these changes the package. They change what the project can claim.

## 2. Priority 1 — Linux musl (`linux-x64-musl`, `linux-arm64-musl`)

**Status (2026-11-22): implemented, awaiting CI evidence.** The loader map, the
packaging and release scripts, the CI `artifacts-musl` job and the ADR-0020
amendment are in the tree. A support claim needs the first green run; the
[platform matrix](../qa/platform-matrix.md) rows stay PENDING until then.

**Why first.** `node:alpine` is the default small Node image, and every musl
install currently fails closed with `CONFIG_NATIVE_UNAVAILABLE`. This is the
single most likely install failure for a new team, and it is a platform, not a
feature.

**How, without breaking ADR-0020's rule.** That ADR says each target is built
and loaded by a runner of that target, never cross-compiled. A musl target fits
if it is built *inside* a musl container: an `ubuntu-24.04` job (and
`ubuntu-24.04-arm`) with `container: node:22-alpine`, `apk add build-base`, a
rustup musl toolchain, then the same `build-artifact` → `verify-platform` →
behaviour harness path the other rows use.

**Work.** Workflow matrix row; `scripts/pack-platform.mjs` metadata
(`os: ["linux"]`, `cpu`, `libc: ["musl"]`); the loader's
`linux-<arch>-musl` keys in `ts/src/loader.ts`; `PLATFORM_PACKAGES` and the
platform matrix doc; release gate amendment to ADR-0020.

**Evidence.** A native musl build that loads and passes the behaviour harness in
the container, a clean install in `node:22-alpine`, and the negative path (a
glibc host asking for the musl build) still failing closed.

**Risk.** `ring` needs a C toolchain on musl, which the container provides;
binary size and build time grow. Nothing in the public contract moves.

## 3. Priority 2 — XOAUTH2 authentication

**Why.** Microsoft 365 and Google Workspace tenants increasingly disable basic
authentication. Today `PLAIN` and `LOGIN` are the only mechanisms, so those
tenants cannot use Sendust at all.

**Contract.** Additive: `auth.accessToken` (a static token or a callback the
facade calls per session) and `auth.method: "xoauth2"`. `user`/`pass` stay as
they are. That is a **minor** release per the
[versioning policy](../engineering/versioning-policy.md).

**Work.** `crates/sendust-core/src/transports/smtp/session.rs` gains the SASL
exchange: `AUTH XOAUTH2 <base64("user=…\x01auth=Bearer …\x01\x01")>`; a `334`
challenge is answered with an empty line before the final `5xx`, which is what
RFC 7628 expects; `AUTH_FAILED` keeps its meaning. Token *refresh* stays the
caller's job — Sendust holds no OAuth client and no secrets store — and the
callback exists so a caller can hand in a fresh token per send.

**Evidence.** Fake-relay tests for success, `334` challenge, an expired token,
and a relay that advertises no `AUTH`; a real-relay probe against a tenant that
accepts XOAUTH2 only; docs and `error-model.md` updated if a code is added.

**Risk.** A token in an error message would be a credential leak; the redaction
audit and `check-secrets` must be extended to cover the new field.

## 4. Priority 3 — DKIM signing

**Why.** Behind an ESP that signs, DKIM is invisible. For a team running its own
MTA, or trying to pass DMARC while relaying through a plain SMTP hop, the client
is the only place to sign — and Nodemailer, Upyo and every hosted SDK already
do it.

**Contract.** Additive: `dkim: { domain, selector, privateKey }`, optional
signing of a fixed header set. **Minor**.

**Work.** Relaxed/relaxed `rsa-sha256` first: canonicalise the chosen headers,
hash the canonicalised body, sign, insert `DKIM-Signature` before `DATA`. New
pinned dependencies (`rsa`, `sha2`; `base64` is already in the tree) go through
the [dependency policy](../engineering/dependency-policy.md). Ed25519 can follow
as a second algorithm.

**Evidence.** Conformance: sign with Sendust and verify with an independent
implementation (`mail-auth` as a dev-dependency, alongside the `mail-parser`
dev-dependency already used for MIME conformance); byte-identical output for a
fixed message; a key that does not parse must fail with a stable code, not panic.

**Risk.** This is the largest of the three — canonicalisation edge cases are
where DKIM implementations break, and it touches the MIME path that currently
has golden tests.

## 5. Priority 4 — ergonomics

| Item | Why | Version |
| --- | --- | --- |
| Mock transport (`@sendust/ts/mock` or an option) | Consumers writing unit tests must run a fake relay today; Upyo ships one | Minor |
| Public `AbortSignal` mapped to bounded close | Callers expect it; the mapping must never report `false accepted`/`false rejected` after `DATA` ([candidates](../research/post-mvp-candidates.md) §3) | Minor |
| SMTPUTF8 | Relay-dependent envelope extension; needs the capability matrix the research note describes | Minor |
| Streaming attachments | Bounded today by reading into memory; streaming needs its own leak and close tests | Minor |

## 6. Still rejected

These remain outside the product, and the 1.0.x line does not reopen them:
provider HTTP transports, template engines, tracking and webhooks, a hosted
service, a persistent queue, a worker or scheduler, queue adapters, URL
attachments, TLS bypass, and SMTP server mode
([vision](../product/vision.md) §4, [candidates](../research/post-mvp-candidates.md) §5).

The one to watch is HTTP provider transports: Upyo, Resend, SendGrid and SES all
have them, and they are the reason an edge-function user cannot choose Sendust.
They would make Sendust a multi-transport client with a provider matrix to
support. If demand ever justifies that, it is a new product line, not a minor
release.

## 7. Sequencing

1. **P0** — clean install, trusted publishing, the Gmail re-measure. No code risk.
2. **1.1.0** — musl + XOAUTH2. Both are additive; together they remove the two
   blockers that stop a real team from adopting.
3. **1.2.0** — DKIM.
4. **Then** ergonomics, by demand.

Each release keeps the current gate discipline: approved commit, pinned facade
hash, five (or seven) platform packages, registry hash re-verified.

## 8. Evidence for this backlog

- Every item names its rationale, the ADR it needs, the version class it lands
  in, and the evidence that would close it.
- The priority order follows the two install-blockers identified in the
  [competitive landscape](../research/competitive-landscape.md): musl and
  OAuth2.
- No item here changes the published `1.0.2` artifact.
