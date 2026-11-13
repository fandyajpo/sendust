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

## Project status

The repository contains the planning and architecture documentation, the completed P03 repository baseline (two-crate workspace, pinned toolchain, quality checks, CI), and the first core behavior of P04: error taxonomy, deterministic validation, address/IDNA handling, the message model, attachment caps, and deterministic MIME composition with wire-size accounting. The lettre-backed SMTP send path and its core fault matrix now exist and pass locally. No native binding or published package exists yet. The modular architecture is approved at design level only; module enforcement and runtime behavior remain unvalidated. Architecture Amendment A01 adds Node.js, Bun, and Deno runtime targets. Architecture Amendment A02 adopts a modular monolith inside the existing two-crate workspace. Runtime, platform, and module-boundary validation remain pending. Discovery (P00), source/API research (P01), and architecture design approval (P02) are complete. QG-02 approves the design baseline, while fake SMTP/TLS behavior, native binding, and runtime support remain mandatory downstream gates. P03 is COMPLETED with an approved QG-03 record and passing CI; P04 is IN PROGRESS with P04-001 to P04-005 in review and the PHASE-04 gate awaiting owner review. See the [architecture approval matrix](docs/architecture/approval-matrix.md) and [QG-02 record](docs/gates/phase-02-architecture-approval.md). Phase/task completion must be backed by evidence under [the definition of done](docs/planning/definition-of-done.md).
