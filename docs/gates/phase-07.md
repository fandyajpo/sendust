# QG-07 — MIME Phase Record

**Decision:** APPROVED — project owner review, 2026-11-14  
**Phase:** P07 — MIME  
**Gate:** PHASE-07  
**Prepared:** 2026-11-14  
**Branch:** `main`  
**Evidence:** local Rust, checker, and runtime commands below, plus the branch CI run.

This record is the review package for PHASE-07. The implementation was written by
an agent, so the approval in section 6 is the required human review step, not a
self-approval.

## 1. Task evidence

| Task | Status | Evidence |
| --- | --- | --- |
| P07-001 | COMPLETED | `message/mod.rs` (one value per custom header, reserved `Message-ID`/`Received`, fixed order) and `mime/headers.rs` (RFC 2047 words that never split a character or exceed 75 characters, lossless folding, encoded-word fallback, unquoted Unicode display names). 10 header unit tests, ordering and exact-byte tests, 3 duplicate-policy tests |
| P07-002 | COMPLETED | `mime/mod.rs` (line-ending normalization, text/HTML/alternative with charset and base64) and `tests/mime_bodies.rs` (byte-exact golden fixtures, empty bodies, line-ending equivalence, no-body rejection) |
| P07-003 | COMPLETED | `attachments/mod.rs` (path stripping, control/empty/`..`/length rejection, case-insensitive duplicate rejection, `type/subtype` tokens) and `mime/mod.rs` (RFC 2231 `filename*` before the ASCII fallback, rendered basename only) |
| P07-004 | COMPLETED | `mime/mod.rs` (`serialize` pure, `serialize_at` with injectable clock adds RFC 5322 `Date` and a deterministic `Message-ID`), `delivery/mod.rs` uses it, wire-size parity tests at the exact cap and one byte over |
| P07-005 | COMPLETED | This record, `tests/mime_conformance.rs` (4 tests parsing the emitted bytes with `mail-parser`), golden fixtures, injection matrix |

## 2. Acceptance checklist

| Check | Result | Evidence |
| --- | --- | --- |
| MIME messages render correctly for all MVP body and metadata cases | PASS | Golden fixtures for text-only and alternative; parser round-trip for text, HTML, alternative, UTF-8 body and attachment |
| Header and filename injection are prevented | PASS | CRLF/LF/NUL/control rejection in header names, values, subject and filenames; path stripping; reserved header set |
| Serialization is deterministic and equals wire-size accounting | PASS | `is_deterministic`, deterministic boundary, parity tests at exact cap and cap − 1, 21/23 MiB large-body test |
| No attachment bytes are read and no SMTP is invoked in this phase | PASS | File descriptors remain policy-checked only (P08 reads); MIME tests are pure |
| An independent parser agrees with the composer | PASS | `tests/mime_conformance.rs` via `mail-parser` 0.11.9 (dev-only) |
| The wire contract is unchanged for transport | PASS | `serialize` output is the same bytes handed to SMTP; BCC never reaches headers |

## 3. Conformance and golden evidence

| Case | Test | Result |
| --- | --- | --- |
| Text-only golden bytes | `mime_bodies::text_only_body_matches_the_golden_fixture` | PASS |
| Alternative golden bytes | `mime_bodies::alternative_body_matches_the_golden_fixture` | PASS |
| HTML-only | `mime_bodies::html_only_body_is_text_html` | PASS |
| Empty-string bodies | `mime_bodies::empty_string_bodies_produce_an_empty_part` | PASS |
| Line-ending normalization | `mime_bodies::line_endings_normalize_to_crlf_before_encoding` | PASS |
| No body rejected | `mime_bodies::a_message_without_a_body_is_rejected` | PASS |
| Parser: text-only round-trip (addresses, subject, Date, Message-ID, BCC absent) | `mime_conformance::parses_a_text_only_message` | PASS |
| Parser: alternative + Unicode subject | `mime_conformance::parses_alternative_bodies_and_unicode_subjects` | PASS |
| Parser: Unicode attachment filename (RFC 2231) | `mime_conformance::parses_an_attachment_with_a_unicode_filename` | PASS |
| Parser: UTF-8 body without loss | `mime_conformance::parses_utf8_bodies_without_loss` | PASS |
| Header injection fixtures | `message` and `mime` unit tests | PASS |
| Filename injection/path fixtures | `attachments` unit tests | PASS |

The conformance tests found one real interop issue: `filename*` after the ASCII
fallback is read as the fallback by a first-match parser, so the parameter order
is now `filename*` first, per RFC 6266 preference.

## 4. Security review

- Header values and names reject CRLF, bare LF, NUL and other control characters
  before any byte is produced; the subject is re-validated at composition.
- Folding never inserts characters into the value: it replaces an existing space
  with CRLF + space, so unfolding restores the exact text; a token that cannot be
  represented legally is re-encoded instead of split.
- Attachment filenames are display names: the last path component is kept, so no
  local directory or traversal reaches the wire; duplicate names are refused.
- BCC appears only in the SMTP envelope: the parser test asserts it never appears
  in the serialized bytes.
- `mail-parser` is a dev-dependency; it is absent from the production graph and
  from the shipped artifact.

## 5. Executed evidence

Run on macOS with the pinned toolchain:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
node scripts/check-architecture.mjs
node scripts/check-docs.mjs
node scripts/check-secrets.mjs
npm run build:native
npm --workspace ./ts run test
node tests/runtime/smoke.mjs
bun tests/runtime/smoke.mjs
deno run --allow-ffi --allow-read --allow-net tests/runtime/smoke.mjs
cargo tree -e normal -p sendust-core
```

Result: 143 core unit tests, 6 MIME body fixtures, 4 conformance tests, 9 harness,
6 client, 18 fault, 6 session tests pass; every checker passes; `cargo tree -e
normal` shows no third-party SMTP engine and no `mail-parser`.

## 6. Decision

**APPROVED.** The project owner reviewed this record and its evidence on
2026-11-14 and approved the PHASE-07 gate. The implementation was written by an
agent, so this approval is the required human review step, not a self-approval:
the reviewer is the project owner and the evidence is the CI run on `main` plus
the local commands above.

Approval covers P07 only. It does not approve P08 or any later gate; a failed
compile, test, checker or runtime result on `main` reopens this gate.

## 7. Deferred (recorded, not claimed)

- `mail-parser` is a dev-dependency only; no parser ships.
- Attachment bytes are read in P08; this phase owns metadata and rendering only.
- The public TypeScript declaration surface is P09 scope.
- Legacy clients without RFC 2231 support see the ASCII fallback filename; that
  is intentional and covered by the conformance test.
