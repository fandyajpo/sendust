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
- One message per `send()` call and one active send per transport.
- Explicit, bounded lifecycle and resource limits.
- A successful result means the SMTP relay accepted responsibility, not that a mailbox received the message.
- No automatic retry, connection pooling, internal queue, worker, or background delivery in the current product scope.
- Queueing, scheduling, and retry orchestration remain the responsibility of the consuming application. Sendust does not provide queue adapters or queue-specific abstractions.

## Documentation

The product definition, requirements, architecture, ADRs, engineering policies, phase plans, and quality gates live in [docs/README.md](docs/README.md). All coding agents and contributors must follow [AGENTS.md](AGENTS.md) and the [AI implementation contract](docs/engineering/ai-implementation-contract.md). Exact runtime/tool versions and validation commands are defined in the [toolchain baseline](docs/engineering/toolchain-baseline.md). The repository-wide static documentation findings are recorded in the [documentation audit](docs/audits/2026-11-13-documentation-audit.md).

## Try a real send

Two runnable entry points exist today. Both read the relay from the environment,
so no credential lives in the repository, and both report relay acceptance only,
never a delivery claim.

Both read `.env` in the repository root (real environment variables win), so:

```text
cp .env.example .env
# edit .env with your relay and recipients

cargo run -p sendust-core --example send_email   # Rust core

npm ci
npm run build:native
node examples/send_email.mjs                     # Node-API binding
```

Both take `SENDUST_HOST`, `SENDUST_PORT` (default 465), `SENDUST_SECURITY` (`tls`
or `starttls`), optional `SENDUST_USER`/`SENDUST_PASS`, `SENDUST_FROM`,
`SENDUST_TO` (comma separated), `SENDUST_SUBJECT`, and `SENDUST_TEXT`. The Rust
example also accepts `SENDUST_EXTRA_ROOT`, a PEM trust anchor for local test
relays. `npm --workspace ./ts run test` runs the TypeScript and binding tests
against the staged addon.

## Project status

The repository contains the planning and architecture documentation, the completed P03 repository baseline (two-crate workspace, pinned toolchain, quality checks, CI), and the first core behavior of P04: error taxonomy, deterministic validation, address/IDNA handling, the message model, attachment caps, and deterministic MIME composition with wire-size accounting. The SMTP send path, its core fault matrix, the Node-API binding, and the runtime-neutral TypeScript facade now exist and pass locally and in CI. ADR-0017 selects a Sendust-owned SMTP engine: the adapter now delivers through the native session, and lettre survives only as a dev-only differential oracle. No npm package is published yet. The modular architecture is approved at design level only; module enforcement and runtime behavior remain unvalidated. Architecture Amendment A01 adds Node.js, Bun, and Deno runtime targets. Architecture Amendment A02 adopts a modular monolith inside the existing two-crate workspace. Runtime, platform, and module-boundary validation remain pending. Discovery (P00), source/API research (P01), and architecture design approval (P02) are complete. QG-02 approves the design baseline, while fake SMTP/TLS behavior, native binding, and runtime support remain mandatory downstream gates. P03 is COMPLETED with an approved QG-03 record and passing CI; P04 and P05 are COMPLETED with approved gate records. See the [architecture approval matrix](docs/architecture/approval-matrix.md) and [QG-02 record](docs/gates/phase-02-architecture-approval.md). Phase/task completion must be backed by evidence under [the definition of done](docs/planning/definition-of-done.md).
