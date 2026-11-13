# Risk Registry

**Status:** planning baseline. Risks with a structural mitigation are `PLANNED`, not `CLOSED`, until evidence exists.

Likelihood: Low / Medium / High. Impact: Low / Medium / High / Critical.

## Technical risks

| ID | Risk | Likelihood | Impact | Mitigation | Owner phase | Status |
| --- | --- | --- | --- | --- | --- | --- |
| R-001 | lettre lower-level RCPT API behavior differs from inspected source during mixed-recipient transaction | Medium | High | P01 source audit selected AsyncSmtpConnection; P06-003 must verify one 250/one 550 produces exactly one DATA transaction for the accepted subset | P06 | OPEN |
| R-002 | Internal SMTP socket sub-stage timing is not observable through lettre public calls | Low | Medium | Approved design reports only coarse public-call boundaries; never infer socket sub-stages; validate deadline behavior in P10-002 | P10 | MITIGATED |
| R-003 | Source-reviewed rustls `ring` + `webpki-roots` feature graph fails Cargo compilation | Medium | High | Exact feature graph is source-reviewed in P01; prove it with cargo check and cargo tree at P03-002 before implementation depends on it | P03 | OPEN |
| R-004 | napi-managed Tokio runtime conflicts with lettre executor expectations at runtime | Medium | High | Pin napi 3.16.2 with explicit async feature, use the managed Tokio 1 runtime, and validate real SMTP execution at P05-003 | P05 | OPEN |
| R-005 | Node-API 8 ABI floor proves insufficient for a required napi feature | Low | Medium | napi 3.16.2 exposes napi8 and async features; compile and runtime-test the pinned set at P05/P14 before claiming support | P05, P14 | OPEN |
| R-006 | Borrowed JS buffer is retained across async and mutated or detached | Medium | Critical | Copy every buffer/string before await; mutation/detachment tests; ownership audit | P05, P11 | PLANNED |
| R-007 | Rust panic unwinds across FFI and aborts the process | Medium | Critical | Use `Result`; `catch_unwind` at boundaries; avoid `panic = "abort"`; panic tests | P04, P05, P11 | PLANNED |
| R-008 | Transport state race allows a second send or a send during close | Medium | High | Explicit state machine; single-send slot; BUSY fast-fail; race matrix | P04, P10 | PLANNED |
| R-009 | Close depends on GC/finalizers and never runs | Medium | High | Explicit close only; no GC-triggered close; forced-GC tests | P09, P10 | PLANNED |
| R-010 | Timeout budgets interact and exceed total or close limits | Medium | High | Single absolute deadline; close > total; injected-clock boundary tests | P04, P10 | PLANNED |
| R-011 | MIME composition is non-conformant (encoding, folding, boundaries) | Medium | High | Independent parser conformance; golden fixtures; RFC 2047/2231 tests | P07 | PLANNED |
| R-012 | Wire-size accounting diverges from actual serialized bytes | Medium | High | Compose, serialize, then measure exact bytes; reject before DATA; parity tests | P07, P08 | PLANNED |
| R-013 | Attachment read follows a symlink or opens a special file | Medium | High | Opt-in FS; no-follow open; fstat regular check; reject others | P08, P11 | PLANNED |
| R-014 | Attachment file grows during read and exceeds caps | Medium | Medium | Read cap + 1 byte; reject overflow; never trust pre-read metadata | P08 | PLANNED |
| R-015 | Peak memory exceeds expectations because inputs are copied and serialized | Medium | Medium | Enforce caps before large copies; measure peak; document worst case | P08, P13 | OPEN |
| R-016 | Native loader selects wrong architecture or libc binary | Medium | High | Explicit resolution order; missing/wrong-binary errors; per-target install tests | P14 | PLANNED |
| R-017 | Partial npm publication leaves root package referencing missing platform packages | Medium | Critical | Artifact-completeness gate; exact version sync; documented recovery; post-publication verification | P16, P17 | PLANNED |
| R-018 | Non-atomic npm publication cannot be rolled back | High | High | Treat as production change; immutable versions; deprecate rather than replace; recovery procedure | P16, P17 | PLANNED |
| R-019 | Support matrix overpromises Node or platform compatibility | Medium | High | Declare only CI-tested targets and versions; explicit unsupported list; docs accuracy gate | P14, P15 | PLANNED |
| R-020 | Enterprise users need OS private CAs, but `webpki-roots` ignores them | Medium | Medium | Document limitation; evaluate platform verifier post-MVP | P01, P18 | OPEN |

| R-046 | Module boundaries erode and domain code imports SMTP/runtime implementation details | Medium | High | Explicit module ownership, narrow facade/ports, static dependency checks at P03/P04 and audit at P12 | P03, P04, P12 | PLANNED |
| R-047 | Premature generic transport/plugin abstraction adds complexity without a second MVP adapter | Medium | Medium | Keep one internal delivery port only where justified; SMTP sole MVP adapter; prohibit public registry | P04, P06 | PLANNED |

## Security risks

| ID | Risk | Likelihood | Impact | Mitigation | Owner phase | Status |
| --- | --- | --- | --- | --- | --- | --- |
| R-021 | Certificate verification can be bypassed through an option or env var | Low | Critical | No bypass code path; source audit; negative tests; public API review | P06, P11 | PLANNED |
| R-022 | Header or envelope injection via CRLF or control characters | Medium | High | Reject control characters in all fields; reserved header protection; injection matrix | P04, P07, P11 | PLANNED |
| R-023 | Credentials leak into errors or logs | Medium | Critical | Redaction rules; serialized-error scans; no password in details; log audit | P09, P11 | PLANNED |
| R-024 | Message body or attachment bytes leak into errors or logs | Low | High | Redaction; marker scans; no body content in diagnostics | P09, P11 | PLANNED |
| R-025 | Absolute local paths leak in attachment errors | Medium | Medium | Redact paths; scan tests | P08, P09, P11 | PLANNED |
| R-026 | Dependency supply-chain compromise or unpatched advisory | Medium | High | Pin versions; license/advisory review; SBOM; minimal dependency set | P03, P11, P16 | PLANNED |
| R-027 | SSRF or network access through URL attachments | Low | High | No URL attachment type; no fetch code; negative test | P08, P11 | PLANNED |
| R-028 | Denial of service via oversized inputs or unbounded close | Medium | Medium | Hard caps; finite timeouts; bounded close drain; memory measurement | P08, P10, P13 | PLANNED |
| R-029 | `catch_unwind` does not cover all panic paths | Medium | High | Keep recoverable failures in `Result`; test panic behavior; document limits | P04, P05, P11 | OPEN |
| R-030 | Tests contact a real relay or use real credentials | Low | High | Fake SMTP/TLS only; network-denial test; secret scan | P03, P12 | PLANNED |

## Compatibility and product risks

| ID | Risk | Likelihood | Impact | Mitigation | Owner phase | Status |
| --- | --- | --- | --- | --- | --- | --- |
| R-031 | Node 26 is pre-release and cannot be claimed | Medium | Low | Treat as candidate; optional for MVP; label unsupported if not testable | P01, P14 | OPEN |
| R-032 | musl targets cannot be built or runtime-tested in CI | Medium | Medium | Decide in P14-003; drop target or label unsupported; never silently claim | P14 | OPEN |
| R-033 | Windows MSVC runtime dependency surprises consumers | Low | Medium | Document runtime requirement; clean-install test | P14, P15 | OPEN |
| R-034 | Documentation implies delivery or automatic retry | Medium | High | Semantic contract; prohibited-claims checklist; docs accuracy gate | P15 | PLANNED |
| R-035 | Scope creep adds pooling, retry, templates, provider APIs, or queue/job infrastructure | Medium | High | Feature matrix; non-goals; gate review; queue/job work requires a separate product decision and is not implicitly eligible for P18 | All gates | PLANNED |
| R-036 | TypeScript wrapper and Rust core drift in status/error semantics | Medium | High | Single mapping source; golden tests; no wrapper reinterpretation | P09, P12 | PLANNED |
| R-037 | Dual ESM/CJS packaging creates duplicate class identities or native loads | Low | Medium | Single shared implementation; mixed-format test; loader cache | P14 | PLANNED |
| R-038 | Premature optimization destabilizes correctness | Medium | Medium | Budgets before measurement; optimize only measured hotspots; re-run QA | P13 | PLANNED |
| R-039 | New implementation workspace lacks source conventions, leading to inconsistent structure | Low | Medium | Phase 03 establishes workspace, Rust MSRV, dependency pins, and policies before implementation | P03 | PLANNED |
| R-040 | A phase is marked complete without evidence | Medium | High | Definition of done; gate records; registry status sync | All gates | PLANNED |
| R-041 | Bun Node-API implementation differs from Node in async/lifecycle behavior | Medium | High | Sendust-owned blocking native addon and lifecycle tests on pinned Bun versions | P05, P14 | OPEN |
| R-042 | Deno native addon loading fails due to package layout or FFI permission behavior | Medium | High | Clean-consumer Deno tests with local node_modules and --allow-ffi; no lifecycle scripts | P03, P14 | OPEN |
| R-043 | TypeScript wrapper assumes Node-only globals or module resolution | Medium | High | Runtime-neutral wrapper audit and identical API contract tests | P03, P05, P09 | OPEN |
| R-044 | Runtime × OS × CPU × libc matrix is only partially tested | High | High | Publish only combinations with blocking CI, clean install and behavioral evidence | P14, P15, P17 | OPEN |
| R-045 | Native loader generated by tooling is not portable across all three runtimes | Medium | High | Audit generated loader; isolate minimal runtime-specific dispatch if required | P03, P05, P14 | OPEN |

## Top risks to watch

1. **R-001** — recipient partial-failure observability. It directly determines the public result contract and is on the critical path.
2. **R-003** — TLS feature composition. It can invalidate the build before implementation.
3. **R-008 / R-010** — lifecycle and timeout races. They are hard to reproduce and damaging in production.
4. **R-017 / R-018** — partial publication and immutable versions. Recovery is manual and public.
5. **R-023** — credential leakage. A single mistake is a critical security incident.

## Review rules

- The owner phase reviews its risks at its gate.
- A risk moves to `CLOSED` only with linked evidence.
- New risks are added with an owner phase before the phase gate closes.
- Accepted residual risks require a written rationale and a post-MVP or operational mitigation.
- R-021, R-023, and R-026 cannot be waived; they must be closed or the release is blocked.
