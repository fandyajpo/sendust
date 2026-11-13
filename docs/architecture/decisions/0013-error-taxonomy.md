# ADR-0013 — Error taxonomy and result contract

**Status:** ACCEPTED for architecture; implementation evidence required at P04-001, P05-004, P09-003, and P11-004.  
**Date:** 2026-11-13.

## Context

Callers need to branch on failure without parsing messages. The same failure can be classified as validation, limit, TLS, authentication, protocol, timeout, lifecycle, or internal. A single generic error loses the information needed for safe retry and operational diagnosis. Conversely, treating a relay rejection as an exception makes normal delivery outcomes awkward.

## Decision

- Delivery outcomes are **return values**: `accepted`, `rejected`, `unknown`, with recipient statuses.
- Operational failures are **errors** with a stable `code`, `category`, and optional redacted `details`.
- Error classes are stable and catchable; codes are append-only within a major version.
- A definitive MAIL/DATA/final-DATA refusal, or an envelope where all RCPT commands are refused, is an outcome (`rejected`), not an exception. A single RCPT refusal in a mixed envelope is a recipient-level `rejected` result while accepted recipients continue through one DATA transaction. Configuration, TLS, authentication, timeout, lifecycle, and internal failures are errors.
- Post-DATA ambiguity is an outcome (`unknown`), not an error, because it is a delivery-semantics result. A timeout after DATA is represented as `unknown` with timeout metadata where possible. An explicit negative final SMTP reply is a known `rejected` outcome.
- Panics never cross FFI; they map to `INTERNAL`.

## Alternatives

1. **Everything is an exception.** Forces callers to use try/catch for normal delivery outcomes and encourages retry on definitive rejection.
2. **Everything is a result union.** Hides programming errors and makes validation failures easy to ignore.
3. **Raw lettre errors exposed.** Couples the public API to the dependency and leaks internal types.
4. **String codes only.** Prevents `instanceof` and structured handling.

## Consequences

- The public contract is explicit and testable.
- Two channels must be documented clearly to avoid caller confusion.
- Every new code needs a test and documentation.
- Redaction is applied at the mapping boundary.
- Changing a code or status meaning is a breaking change.

## Evidence required

- Proposed status/error tables in [error-model.md](../error-model.md), pending P02-005 approval.
- Golden tests for every code and outcome in P09-003/P09-005.
- Redaction scans in P11-004.
