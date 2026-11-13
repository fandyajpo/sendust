# Sendust

**Standalone Native Email Delivery Engine**

Sendust is a Rust-powered native email delivery engine for Node.js applications. It exposes a narrow TypeScript API backed by a Node-API adapter and a Node-independent Rust core. Sendust submits transactional email through an SMTP relay configured by the consuming application.

Sendust is an embeddable library, not an SMTP server, hosted email service, mailbox platform, campaign manager, or job scheduler.

## Product boundaries

- SMTP-first: implicit TLS and STARTTLS, with certificate and hostname verification.
- Native Rust core; thin Node-API adapter; TypeScript public API.
- One message per `send()` call and one active send per transport.
- Explicit, bounded lifecycle and resource limits.
- A successful result means the SMTP relay accepted responsibility, not that a mailbox received the message.
- No automatic retry, connection pooling, internal queue, worker, or background delivery in the current product scope.
- Queueing, scheduling, and retry orchestration remain the responsibility of the consuming application. Sendust does not provide queue adapters or queue-specific abstractions.

## Documentation

The product definition, requirements, architecture, ADRs, engineering policies, phase plans, and quality gates live in [docs/README.md](docs/README.md).

## Project status

This repository currently contains planning and architecture documentation. No application implementation or published package is claimed. Phase and task completion must be backed by evidence under [docs/planning/definition-of-done.md](docs/planning/definition-of-done.md).
