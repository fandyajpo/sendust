# Sendust — Constraint Ledger

**Status:** audit baseline; owner sign-off pending  
**Source of truth:** [requirements.md](../product/requirements.md)  
**Date:** 2026-11-13

This ledger maps every authoritative constraint C-01 through C-40 to its architecture decision, enforcement work, and evidence. “Source-reviewed” means API/source documentation was inspected; it does not mean the future implementation has passed tests.

| ID | Constraint summary | Decision / ADR | Enforcement and evidence | Audit status |
| --- | --- | --- | --- | --- |
| C-01 | Exactly two Rust crates; core has no Node linkage | ADR-0001 | P02-001, P03-001, P04-005; dependency-tree check | Architecture defined |
| C-02 | TS → napi → core → lettre; dependencies point downward | ADR-0001/0002 | P02-001, P03-001, P09-001; dependency graph | Architecture defined |
| C-03 | Sendust-owned SMTP engine on trusted TLS/I/O primitives; no third-party SMTP engine | ADR-0017 (supersedes ADR-0002) | P06-001..P06-005; cargo tree check | Native engine pending P06 |
| C-04 | napi-managed Tokio 1; no second runtime | ADR-0004 | P01-004, P05-003; runtime integration tests | Source-reviewed; napi async feature enables tokio_rt; runtime probe remains |
| C-05 | Node-API 8 | ADR-0004 | P01-004, P05-001, P14-004; ABI matrix | napi8 feature explicitly required; runtime tests remain |
| C-06 | Node 22/24/26 are candidates only | ADR-0004/0011 | P01-004, P14-004, P15-001; blocking CI matrix | Candidate-only |
| C-07 | rustls + ring + webpki-roots; verification on; TLS 1.2 minimum | ADR-0003 | P01-002, P06-002, P11-002; certificate fixtures | Feature graph source-reviewed; compile/fixtures remain |
| C-08 | One active send per transport; immediate BUSY | ADR-0006 | P02-003, P04-004, P09-004, P10-001; race tests | Architecture defined |
| C-09 | Explicit open → closing → closed; bounded idempotent close | ADR-0006 | P02-003, P04-004, P09-004, P10-001; lifecycle tests | Architecture defined; cancellation edge clarified |
| C-10 | Connect ≤10s, operation ≤30s, total ≤120s, close ≤125s; values may only be lowered | ADR-0007 | P01-003, P02-003, P04-004, P10-002; injected-clock tests | Coarse-stage model source-reviewed |
| C-11 | JS-owned input copied before async | ADR-0008 | P02-004, P05-002, P11-004; mutation/detachment tests | Requires sync conversion + copy correction |
| C-12 | Filesystem attachments opt-in; regular files only | ADR-0014 | P02-004, P08-001/002, P11-003; file-type matrix | Architecture defined |
| C-13 | 100 recipients; 10/20/30 MiB attachment/wire caps | ADR-0009 | P02-004, P04-003, P08-003; boundary tests | Architecture defined |
| C-14 | ASCII local parts, IDNA domains, Unicode display/body | ADR-0010 (policy boundary) | P04-002, P07-001/004; address/MIME tests | Policy defined; implementation tests remain |
| C-15 | Relay acceptance only; no delivery claim; unknown is not retried | ADR-0005/0013 | P02-002, P06-004, P09-002; SMTP traces | Semantics source-reviewed |
| C-16 | No pool, retries, queue, worker, scheduler, persistence, adapter | ADR-0006 | P02-003, P09-004; API and source audits | Confirmed product boundary |
| C-17 | No public AbortSignal in MVP; internal bounded close cancellation is required | ADR-0006 | P02-003, P09-004, P18-002; lifecycle tests | Architecture defined |
| C-18 | Per-recipient RCPT visibility | ADR-0005 | P01-003, P02-002, P06-003, P09-002; mixed-RCPT trace | Public API source-reviewed; fake-relay test remains |
| C-19 | Redact credentials, body, attachment bytes, absolute paths | ADR-0013/0014 | P02-002, P09-003, P11-004; marker scans | Policy defined |
| C-20 | Stable machine-readable errors; no panic across FFI | ADR-0013 | P02-002, P04-001, P05-004, P09-003; golden tests | Contract defined |
| C-21 | Local deterministic fake SMTP/TLS only; no real relay or secrets | Test-seam decision | P00-004, P03-004, P12-004; network-denial test | Policy defined |
| C-22 | ESM and CommonJS with identical behavior | ADR-0012 | P09-001, P14-002/004; mixed-format test | Proposed; later packaging decision |
| C-23 | Reproducible, traceable release artifacts | ADR-0011 | P16-002/004, P17-001/004; provenance | Later release gate |
| C-24 | Completion requires evidence, not documentation claims | Definition of Done | Every phase gate; evidence links | Policy defined |
| C-25 | Single source of truth across requirements, tasks, ADRs, matrix | Documentation policy | P15-005; link and consistency audit | Policy defined |
| C-26 | Support claims match tested Node/platform matrix | ADR-0011 | P14-003/004, P15-001; CI matrix | Later distribution gate |
| C-27 | No Rust toolchain or postinstall download for consumers | ADR-0011 | P14-002/004, P17-004; clean install | Later distribution gate |
| C-28 | Implicit TLS and required STARTTLS only; no plaintext | ADR-0003 | P06-002, P11-002; negative tests | Architecture defined |
| C-29 | Reject CRLF/control chars; protect reserved headers | ADR-0010/0014 | P04-002, P07-001, P11-003; injection matrix | Policy defined |
| C-30 | Standalone delivery engine; no hosted platform or job system | Product vision / ADR-0006 | P00-001, P15-005; scope audit | Confirmed product boundary |

| C-31 | Node.js, Bun, Deno are required MVP runtimes | ADR-0015 | P03, P05, P14; blocking runtime CI | Design approved; tests pending |
| C-32 | One shared Node-API addon; exactly two Rust crates | ADR-0001/0015 | P03, P05; dependency and runtime tests | Design approved; tests pending |
| C-33 | Runtime-neutral TypeScript API and isolated loader | ADR-0015 | P03, P05, P09; runtime contract tests | Design approved; tests pending |
| C-34 | Deno local node_modules and explicit --allow-ffi; no install scripts | ADR-0015 | P03, P14, P15; clean install | Design approved; tests pending |
| C-35 | Support claims require exact runtime × platform evidence | ADR-0011/0015 | P14, P15, P17; blocking CI matrix | Design approved; tests pending |
| C-36 | Explicit MVP OS/CPU/ABI candidate baseline | ADR-0011/0015 | P03, P14; build/load matrix | Design approved; tests pending |
| C-37 | Modular monolith within existing two-crate workspace | ADR-0016 | P03/P04; structure audit | Design approved; tests pending |
| C-38 | Explicit core module ownership and acyclic dependencies | ADR-0016 | P03/P04; architecture tests | Design approved; tests pending |
| C-39 | SMTP-only adapter and narrow internal delivery port | ADR-0002/0016 | P04/P06; contract and adapter tests | Design approved; tests pending |
| C-40 | Module boundary enforcement and test ownership | ADR-0016 | P03/P04/P12; CI checks | Design approved; tests pending |

## Open validation register

| Question | Resolution from source review | Remaining evidence | Owner task |
| --- | --- | --- | --- |
| Per-recipient RCPT status | Public AsyncSmtpConnection.command + Error.status() expose each SMTP reply | Mixed-recipient fake-relay test and behavior test | P06-003 |
| SMTP stage attribution | Sendust can label coarse operations around public lettre methods; lettre does not expose all internal sub-stages | Injected timeout/fault tests | P10-002 |
| TLS features | Exact feature graph is expressible with defaults disabled | Cargo build and five negative TLS fixtures | P03-002, P06-002, P11-002 |
| Node runtime | NAPI-RS async/AsyncBlock runs on managed Tokio; Node-API 8 is ABI-compatible with candidates | Actual runtime/GC/exit CI matrix | P05-003, P14-004 |
| Buffer ownership | Buffer retains JS memory; sync copy into Vec<u8> before returning AsyncBlock is required | Mutation/detachment integration test | P05-002 |

## Gate rule

No item marked “architecture defined” is evidence that implementation exists. The architecture gate may approve the design while retaining implementation tests as explicit downstream acceptance criteria.