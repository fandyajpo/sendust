# Sendust

**Standalone Native Email Delivery Engine**

Sendust is a Rust-powered native email delivery engine for Node.js, Bun, and Deno applications. It exposes a narrow TypeScript API backed by a shared Node-API adapter and a runtime-independent Rust core. Sendust submits transactional email through an SMTP relay configured by the consuming application.

Sendust is an embeddable library, not an SMTP server, hosted email service, mailbox platform, campaign manager, or job scheduler.

## Product boundaries

- SMTP-first: implicit TLS and STARTTLS, with certificate and hostname verification.
- Native Rust core; thin Node-API adapter; runtime-neutral TypeScript public API.
- Modular monolith inside exactly two Rust crates, with strict internal module boundaries.
- Node.js, Bun, and Deno are required MVP runtime targets, subject to Sendust-owned blocking CI.
- Server platform support is limited to explicitly tested OS/CPU/ABI combinations.
- One message per `send()` call and one SMTP transaction per session; concurrent sends are queued and pooled automatically, and a session is reused across sequential messages.
- Explicit, bounded lifecycle and resource limits.
- A successful result means the SMTP relay accepted responsibility, not that a mailbox received the message.
- Automatic in-process queueing, connection pooling, adaptive concurrency, session reuse, and bounded retries for transient failures; no worker process, no scheduler, and no concurrent transaction on one session.
- Queueing, scheduling, and retry policy beyond one transport's lifetime remain the consuming application's responsibility. Sendust does not provide a persistent queue, queue adapters, or queue-specific abstractions.

## Documentation

The product definition, requirements, architecture, ADRs, engineering policies, phase plans, and quality gates live in [docs/README.md](docs/README.md). How the project got from an empty folder to its current release is told in [the Sendust story](docs/releases/story.md). All coding agents and contributors must follow [AGENTS.md](AGENTS.md) and the [AI implementation contract](docs/engineering/ai-implementation-contract.md). Exact runtime/tool versions and validation commands are defined in the [toolchain baseline](docs/engineering/toolchain-baseline.md). The repository-wide static documentation findings are recorded in the [documentation audit](docs/audits/2026-11-13-documentation-audit.md) and the [full repository audit](docs/audits/2026-11-15-full-repository-audit.md).

## Try a real send

Two runnable entry points exist today. Both read the relay from the environment,
so no credential lives in the repository, and both report relay acceptance only,
never a delivery claim.

Both read `.env` in the repository root (real environment variables win), so
start from the committed example and pick one relay profile:

```text
cp .env.example .env
# edit .env with your relay and recipients

cargo run -p sendust-core --example send_email   # Rust core

npm ci
npm run build:native
node examples/send_email.mjs                     # Node-API binding
```

The examples refuse values that still look like the shipped placeholders, and
for a `gmail.com` relay they strip the spaces Google shows in an app password.
For Gmail, create an app password at
`https://myaccount.google.com/security/apppasswords` and set
`SENDUST_USER`/`SENDUST_PASS` to the same account; `SENDUST_FROM` must be that
address or an alias you own. Both take `SENDUST_HOST`, `SENDUST_PORT` (default
465), `SENDUST_SECURITY` (`tls` or `starttls`), optional
`SENDUST_USER`/`SENDUST_PASS`, `SENDUST_FROM`, `SENDUST_TO` (comma separated),
`SENDUST_SUBJECT`, and `SENDUST_TEXT`. The Rust example also accepts
`SENDUST_EXTRA_ROOT`, a PEM trust anchor for local test relays.
`npm --workspace ./ts run test` runs the TypeScript and binding tests against
the staged addon.

## Project status

The repository contains the planning and architecture documentation, the completed P03 repository baseline, and the full MVP delivered across P04–P18. The native SMTP engine (ADR-0017), the Node-API binding, and the runtime-neutral TypeScript facade are complete, validated on Node.js, Bun, and Deno, and distributed on seven native targets, including Alpine (musl). `0.1.0-rc.1` was published on 2026-11-18, `1.0.0` on 2026-11-19, `1.0.1` on 2026-11-21, `1.0.2` on 2026-11-21, `1.1.0` on 2026-11-22, and `1.1.1` on 2026-11-23; each is tagged (`v0.1.0-rc.1`, `v1.0.0`, `v1.0.1`, `v1.0.2`, `v1.1.0`, `v1.1.1`). Version `1.1.1` is a metadata patch that moved publication to npm trusted publishing, so every release now carries a provenance attestation and no npm token ([gate record](docs/gates/release-1.1.1.md), [release notes](docs/releases/1.1.1.md)). Version `1.1.0` adds the Linux musl targets and XOAUTH2 ([gate record](docs/gates/release-1.1.0.md), [release notes](docs/releases/1.1.0.md)). Version `1.0.2` is a resource patch: an idle relay session is now released instead of being held open, and a rejected `delivery` option no longer leaves a native transport behind ([gate record](docs/gates/release-1.0.2.md), [release notes](docs/releases/1.0.2.md)). Version `1.0.1` is a performance patch over `1.0.0`: the pool fills to the work already waiting, and a completed transaction costs one round trip less ([gate record](docs/gates/release-1.0.1.md), [release notes](docs/releases/1.0.1.md)). Version `1.0.0` adds the autonomous delivery engine (ADR-0021: bounded queue, transport pool, adaptive concurrency, bounded retries) and SMTP session reuse (ADR-0022), and it is the first stable release; its evidence is the [release gate record](docs/gates/release-1.0.0.md), the [benchmark](docs/qa/benchmark-nodemailer.md), and the [release notes](docs/releases/1.0.0.md). The `latest` dist-tag resolves `1.1.1`, so `npm install @sendust/ts` installs the current stable release; `next` still resolves `1.0.0`. How it compares with Nodemailer, Upyo, EmailJS and the hosted-API SDKs is in the [competitive landscape](docs/research/competitive-landscape.md). See the [architecture approval matrix](docs/architecture/approval-matrix.md) and [QG-02 record](docs/gates/phase-02-architecture-approval.md); phase and task completion is backed by evidence under [the definition of done](docs/planning/definition-of-done.md).

## License

Dual licensed under either of:

- Apache License, Version 2.0 — [LICENSE-APACHE](LICENSE-APACHE)
- MIT license — [LICENSE-MIT](LICENSE-MIT)

at your option. Contributions are accepted under the same terms.
