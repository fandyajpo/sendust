# QG-04 — Rust Core Phase Record

**Decision:** APPROVED — project owner review, 2026-11-14  
**Phase:** P04 — Rust core  
**Gate:** PHASE-04  
**Prepared:** 2026-11-14  
**Branch:** `feat/p04-core-errors` (merged as `56e789c`)  
**Evidence:** 14/14 CI checks on branch head `75fc901`; 7/7 CI checks on the merge commit `56e789c` on `main`; local Rust and checker evidence recorded below.

## 1. Task evidence

| Task | Status | Evidence |
| --- | --- | --- |
| P04-001 | COMPLETED | `crates/sendust-core/src/errors/mod.rs` (32 stable codes, categories, whitelisted details, stage enum) and `crates/sendust-core/src/validation/mod.rs` (eight-stage deterministic precedence); 13 unit tests including the documentation parity table and a redaction scan |
| P04-002 | COMPLETED | `crates/sendust-core/src/address/mod.rs` (ASCII local parts, IDNA/UTS #46 strict domains, Unicode display names) and validated header newtypes; 35 unit tests including IDNA, punycode, overlong, malformed, CRLF/NUL/tab injection |
| P04-003 | COMPLETED | `config/mod.rs` (approved hard caps, lower-only), `attachments/mod.rs`, `message/mod.rs`, `mime/mod.rs` (deterministic serializer, RFC 2047, hard folding, BCC exclusion, exact wire cap); 69 unit tests |
| P04-004 | COMPLETED | `lifecycle/mod.rs` (`open -> closing -> closed`, one-send reservation, immediate BUSY, idempotent close, bounded drain) and the 10/30/120/125 s budgets; state and deadline tests use explicit `Instant`s, no sleeps |
| P04-005 | COMPLETED | `transports/smtp/mod.rs` (lettre 0.11.23 `AsyncSmtpConnection`) and `tests/smtp_faults.rs` (10 fault-matrix tests); 94 core unit tests, 9 harness self-tests, 10 SMTP fault tests |

## 2. Acceptance checklist

| Check | Result | Evidence |
| --- | --- | --- |
| Core has no Node linkage | PASS | `scripts/check-architecture.mjs` rejects `napi`/Node/`lettre` outside `transports/smtp`; CI job passes |
| Domain modules independent of lettre/SMTP/runtime | PASS | Architecture check passes with the adapter the only `lettre` owner; `SendDeadlines` moved to `config` so the adapter depends on domain and port modules only |
| Delivery orchestration behind a narrow port | PASS | `ports/mod.rs` (`DeliveryPort`, `DeliveryRequest`); the SMTP adapter is the only MVP implementation |
| ADR-0005 recipient policy | PASS | `tests/smtp_faults.rs`: mixed 250/550 shares exactly one DATA; all-RCPT refusal issues none; accepted only after final DATA 2xx |
| Unknown after DATA | PASS | Dropped final reply after the terminator yields `unknown`, no retry |
| No retry, pooling, custom SMTP, SMTPUTF8, URL attachments | PASS | No retry path exists; `lettre` owns the protocol; SMTPUTF8 and URL attachments are rejected in core |
| Fault fixtures pass without external network | PASS | All fault tests bind `127.0.0.1` ephemeral ports with committed fixtures |
| Fail-closed TLS | PASS | Untrusted certificate maps to `TLS_CERTIFICATE`; STARTTLS is required, never opportunistic |

## 3. Executed evidence

Run on macOS, `rustc 1.100.0` (pinned toolchain), commit `75fc901`:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
node scripts/check-architecture.mjs
node scripts/check-docs.mjs
node scripts/check-secrets.mjs
```

Result: 94 core unit tests, 9 harness self-tests, 10 SMTP fault tests pass; all three checkers pass.

## 4. Deferred to P06 (recorded, not claimed)

- STARTTLS upgrade against a real relay and the five certificate fixtures (self-signed, expired, wrong host, incomplete chain, malformed) belong to P06-002.
- Per-recipient behaviour against a live relay as a fault matrix belongs to P06-003/P06-004; P04 proves it against the loopback harness.

## 5. Decision

**APPROVED.** The project owner reviewed this record and its CI evidence on
2026-11-14 and authorized P05 to begin. The implementation was written by an
agent, so this approval is the required human review step, not a self-approval:
the reviewer is the project owner and the evidence is the CI run on the branch
head plus the local commands above.

Approval covers P04 only. It does not approve P05, runtime/platform support, or
any later gate; a failed compile, harness, or checker result on `main` reopens
this gate.
