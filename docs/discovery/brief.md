# Sendust — Discovery Brief

**Status:** audit draft; ready for product-owner review  
**Date:** 2026-11-13  
**Product definition:** Standalone Native Email Delivery Engine

## Problem statement

Server-side JavaScript applications running on Node.js, Bun, or Deno need a dependable way to submit transactional email through an SMTP relay they configure and operate. Sendust provides a narrow TypeScript API backed by a native Rust engine so applications can submit one message per call with correct MIME handling, mandatory TLS verification, explicit SMTP outcomes, bounded resource use, and deterministic lifecycle behavior.

**Falsifiable success statement:** Given a valid message and a reachable, correctly configured SMTP relay, a supported Node.js, Bun, or Deno runtime can submit one SMTP transaction and report **accepted** only after receiving a positive final DATA reply, while refusing unsafe input and bounding every network and resource operation.

A positive SMTP reply means the relay accepted responsibility. It does not prove mailbox delivery.

## Primary consumer and personas

| Persona | Job to be done | Success |
| --- | --- | --- |
| Node.js backend developer | Send receipts, password resets, and transactional notifications through an existing relay | Small typed API; no Rust toolchain at install; reliable result semantics |
| Platform engineer | Configure relay credentials, TLS, and shutdown behavior across a Node fleet | Mandatory certificate verification, bounded operations, no hidden retry or queue |
| Library maintainer | Publish and maintain a native npm package | Isolated Rust core, deterministic tests, reproducible platform artifacts |
| Security reviewer | Review handling of credentials, content, filesystem access, and native memory | Redacted diagnostics, bounded inputs, explicit trust boundaries |

## Core scenarios

1. **Successful transactional send:** a service configures an implicit-TLS relay, sends a receipt, receives a positive final DATA response, and gets accepted.
2. **Recipient partial refusal:** a message targets To/CC/BCC recipients; the relay accepts some RCPT commands and rejects others. Sendust reports each explicit RCPT outcome and sends one DATA transaction only to accepted recipients.
3. **Ambiguous final disposition:** the relay may have received the DATA terminator, but the connection drops before a final reply. Sendust returns unknown and does not retry.
4. **Unsafe attachment request:** filesystem access is disabled by default; a path attachment fails before any filesystem read unless the caller explicitly opts in.
5. **Shutdown during send:** close prevents new sends, requests cancellation, and waits within a finite deadline without relying on garbage collection.

## Product boundaries

Sendust is an embeddable native delivery library for server-side JavaScript runtimes (Node.js, Bun, and Deno). It is not:
- an SMTP server, mailbox host, hosted email service, or campaign platform;
- a provider REST API client or inbound IMAP/POP3 implementation;
- a template manager, tracking/analytics system, or web administration product;
- a queue, worker, scheduler, persistence system, or job orchestration framework.

**Queue boundary:** queue/job orchestration is held outside the current roadmap. Sendust exposes a direct send operation only. Queueing, scheduling, persistence, and retry decisions remain caller-owned. No queue abstraction or adapter contract is to be designed.

## MVP success conditions

- A valid send uses one SMTP connection and one SMTP transaction.
- accepted requires a positive final DATA response.
- Explicit SMTP refusal is rejected; indeterminate final disposition is unknown.
- No automatic retry, connection pool, queue, worker, scheduler, or queue adapter.
- TLS verification is mandatory; plaintext SMTP is not supported.
- Every input, timeout, attachment, and wire message is bounded.
- Consumers install a tested prebuilt native package without a Rust toolchain.
- All implementation and release claims are supported by tests and artifact evidence.

## Discovery decisions and handoff

- Use the Rust core + thin Node-API adapter + TypeScript wrapper architecture.
- Prefer lettre 0.11.23; use its lower-level async SMTP connection API where per-recipient RCPT visibility is required. Do not implement SMTP command parsing or TLS ourselves.
- Use napi-rs v3 and its managed Tokio 1 runtime; Node-API 8 is the proposed ABI target.
- Node.js 22, 24, and 26 remain version candidates; exact Bun and Deno versions are pinned at P03. No runtime is supported until blocking CI evidence exists.
- Rustls with ring and webpki-roots is the selected TLS configuration, with certificate and hostname verification mandatory.
- The exact recipient and timeout behavior is documented in [lettre research](../research/lettre-0.11.23-validation.md).
- Runtime behavior, compilation, certificate fixtures, and fault injection remain implementation-phase validation; this discovery record does not claim those tests have run.

## Review

This brief is derived from the authoritative product vision and requirements. Product-owner acceptance is still required before the discovery gate is formally signed.