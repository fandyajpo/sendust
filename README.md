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

The repository currently contains planning and architecture documentation; no application implementation or published package is claimed. Discovery (P00), source/API research (P01), and architecture design approval (P02) are complete. QG-02 approves the design baseline, while Cargo compilation, fake SMTP/TLS behavior, native binding, and runtime support remain mandatory downstream gates. The next phase is P03 Repository. See the [architecture approval matrix](docs/architecture/approval-matrix.md) and [QG-02 record](docs/gates/phase-02-architecture-approval.md). Phase/task completion must be backed by evidence under [the definition of done](docs/planning/definition-of-done.md).
