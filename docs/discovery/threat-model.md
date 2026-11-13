# Sendust — Threat and Operational Boundary

**Status:** architecture baseline; implementation verification pending  
**Date:** 2026-11-13

## Assets

| Asset | Examples | Security property |
| --- | --- | --- |
| SMTP credentials | Passwords, tokens, AUTH challenge material | Confidentiality; never log or serialize |
| Message content | Subject, body, headers, recipients | Confidentiality and integrity |
| Attachments | JS buffers and local files | Confidentiality, bounded memory, integrity |
| Relay identity | Hostname, port, EHLO name, certificate | Authenticity and transport integrity |
| Process resources | Tasks, sockets, file descriptors, memory | Bounded use and deterministic shutdown |
| Native boundary | JS values, Rust allocations, FFI calls | Memory safety and panic containment |
| Release artifacts | .node binaries and npm packages | Integrity, provenance, target correctness |

## Trust boundaries

1. Consumer application → TypeScript wrapper: caller-controlled JavaScript values and configuration.
2. TypeScript wrapper → Node-API adapter: shape conversion, native handles, typed arrays and buffers.
3. Node-API adapter → Rust core: owned values only; no borrowed JS memory may enter async work.
4. Rust core → local filesystem: only when explicitly enabled; opened file descriptor is revalidated.
5. Rust core → SMTP relay/network: untrusted DNS, TCP, TLS, SMTP replies, and remote timing.
6. CI/build → npm registry: toolchain, dependencies, generated binaries, package metadata.

## Threat assumptions

- The consumer chooses and operates the SMTP relay and supplies credentials.
- The relay and network are not trusted to provide timely, well-formed, or truthful responses.
- The caller may pass malformed, oversized, mutated, or adversarial input.
- The library is not a sandbox against a fully compromised host process.
- Tests use only loopback fake SMTP/TLS services and synthetic credentials.

## Abuse-case register

| ID | Abuse case | Required mitigation | Evidence owner |
| --- | --- | --- | --- |
| AB-01 | SMTP MITM or STARTTLS stripping | Implicit TLS or required STARTTLS; never opportunistic fallback; hostname and chain verification | P06-002, P11-002 |
| AB-02 | Expired, self-signed, wrong-host, incomplete-chain, malformed certificate | Fail closed with rustls verifier and negative fixtures | P06-005, P11-002 |
| AB-03 | CRLF/control injection in address, header, filename, or EHLO | Reject control chars; reserved headers cannot be overridden | P04-002, P07-001, P11-003 |
| AB-04 | Oversized recipients, attachment, or wire payload | Hard caps before DATA; deterministic validation order | P04-003, P08-003 |
| AB-05 | Symlink/FIFO/device/directory attachment path | Filesystem opt-in; open with no-follow where supported; fstat descriptor; regular files only | P08-002, P11-003 |
| AB-06 | File changes or grows during read | Open-then-fstat; bounded read plus one-byte overflow probe | P08-002 |
| AB-07 | JS mutates or detaches Buffer after send call | Copy Buffer/typed-array bytes synchronously before AsyncBlock is returned | P05-002, P11-004 |
| AB-08 | Relay stalls at DNS/connect/TLS/greeting/AUTH/MAIL/RCPT/DATA/final response | Outer connect budget, per-operation budget, absolute total deadline, bounded close | P10-002, P10-004 |
| AB-09 | Connection drops after DATA terminator | Return unknown; no automatic retry; drop connection without a DATA-state QUIT attempt | P06-004, P10-001 |
| AB-10 | Concurrent send or send/close race | Synchronous state reservation under a short lock; immediate BUSY; close atomically transitions to closing | P04-004, P10-001 |
| AB-11 | Panic crosses Node-API boundary | Expected failures use Result; unwind-capable build; NAPI boundary catches where supported | P04-001, P05-004, P11-004 |
| AB-12 | Credentials/body/path leak through diagnostics | Structured redaction; no tracing of auth or message payload; marker scans | P09-003, P11-004 |
| AB-13 | External relay/secret used by tests | Loopback-only fake relay; network-denial test; secret scan | P03-004, P12-004 |
| AB-14 | Platform binary mismatch or compromised artifact | Exact target matrix, checksums, SBOM, provenance, clean-install tests | P14-003, P16-002 |

## Operational invariants

- Sendust has no built-in queue, worker, scheduler, persistence, retry engine, or queue adapter.
- One transport accepts one active send; BUSY is immediate.
- One connection is opened per send and is dropped after the transaction.
- No connection is retained while idle.
- Every network operation has a finite timeout.
- close cancels the active send and waits for bounded cleanup; it does not use a garbage-collection finalizer.
- No test contacts an external SMTP relay or includes real credentials.

## Residual risks

- webpki-roots does not automatically trust private enterprise roots installed only in the operating system. This is an intentional MVP trade-off and must be documented.
- Cross-platform no-follow file opening differs by OS; platform-specific tests must prove the chosen implementation.
- Cancellation during the DATA phase is conservative: if the final disposition cannot be observed, the send result is unknown.
- A native package expands supply-chain impact; release provenance and immutable versioned artifacts are mandatory.