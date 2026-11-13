# MVP Feature Matrix

**Authoritative feature classification for Sendust.**  
Requirements: [requirements.md](../product/requirements.md). Scope summary: [mvp-scope.md](../product/mvp-scope.md).  
Every `REQUIRED FOR MVP` row must map to at least one implementation task and at least one test task. P12-001 audits this.

## Classification

- **REQUIRED FOR MVP** — must ship in the first public release.
- **OPTIONAL FOR MVP** — may ship only if it adds no risk and no scope creep; absence is acceptable.
- **POST-MVP** — explicitly deferred; not in the first release.
- **HELD** — deliberately excluded from the current roadmap; not an MVP feature or an active post-MVP candidate. Reconsideration requires a separate product decision.
- **OUT OF SCOPE** — permanently excluded from the product direction.

## SMTP

| ID | Feature | Class | Implementation | Tests | Notes |
| --- | --- | --- | --- | --- | --- |
| F-SMTP-01 | SMTP connection to configured relay | REQUIRED | P06-001 | P06-005, P12-002 | One connection per send |
| F-SMTP-02 | PLAIN authentication | REQUIRED | P06-002 | P06-005, P12-002 | Credentials redacted |
| F-SMTP-03 | LOGIN authentication | REQUIRED | P06-002 | P06-005, P12-002 | Unsupported mechanisms rejected |
| F-SMTP-04 | Implicit TLS | REQUIRED | P06-001, P06-002 | P06-005 | Verification mandatory |
| F-SMTP-05 | STARTTLS | REQUIRED | P06-002 | P06-005 | No plaintext fallback |
| F-SMTP-06 | Connect timeout | REQUIRED | P04-004, P10-002 | P10-004, P12-002 | Default 10 s |
| F-SMTP-07 | Operation timeout | REQUIRED | P04-004, P10-002 | P10-004, P12-002 | Default 30 s |
| F-SMTP-08 | Total send timeout | REQUIRED | P04-004, P10-002 | P10-004, P12-002 | Default 120 s |
| F-SMTP-09 | SMTP response handling and classification | REQUIRED | P06-004 | P06-005, P12-002 | Reply classes mapped to outcomes/errors |
| F-SMTP-10 | Graceful close | REQUIRED | P04-004, P09-004, P10-001 | P10-004, P12-003 | Idempotent, bounded |
| F-SMTP-11 | Certificate and hostname verification | REQUIRED | P06-002 | P06-005, P11-002 | Never disableable |
| F-SMTP-12 | Plaintext SMTP mode | OUT OF SCOPE | — | P11-002 negative test | Security posture |
| F-SMTP-13 | Connection pooling | POST-MVP | P18-003 research only | — | See ADR-0006 |
| F-SMTP-14 | Automatic retry/backoff | POST-MVP | Future ADR/research decision | — | No automatic retry in MVP; unknown outcomes require caller judgment |
| F-SMTP-15 | SMTPUTF8 | POST-MVP | P18-002 research only | — | ASCII local parts in MVP |
| F-SMTP-16 | DKIM signing | POST-MVP | — | — | Post-MVP candidate |
| F-SMTP-17 | `verify()` connection probe | POST-MVP | — | — | Not required to send |
| F-SMTP-18 | Reject unsupported security/auth modes before connect | REQUIRED | P06-002 | P06-005 | Configuration error |

## Message composition

| ID | Feature | Class | Implementation | Tests | Notes |
| --- | --- | --- | --- | --- | --- |
| F-MSG-01 | From with optional display name | REQUIRED | P07-001 | P07-005, P12-002 | Unicode display allowed |
| F-MSG-02 | To recipients | REQUIRED | P07-001, P06-001 | P07-005, P06-005 | Envelope + header |
| F-MSG-03 | CC recipients | REQUIRED | P07-001, P06-001 | P07-005, P06-005 | Envelope + header |
| F-MSG-04 | BCC recipients | REQUIRED | P07-001, P06-001 | P07-005, P06-005 | Envelope only; absent from headers |
| F-MSG-05 | Reply-To | REQUIRED | P07-001 | P07-005 | Multiple allowed per contract |
| F-MSG-06 | Subject with Unicode encoding | REQUIRED | P07-001 | P07-005 | RFC 2047 |
| F-MSG-07 | Plain-text body | REQUIRED | P07-002 | P07-005, P12-002 | Empty body allowed |
| F-MSG-08 | HTML body | REQUIRED | P07-002 | P07-005, P12-002 | Not sanitized; documented |
| F-MSG-09 | Validated custom headers | REQUIRED | P07-001 | P07-005, P11-003 | Reserved names rejected |
| F-MSG-10 | `multipart/alternative` | REQUIRED | P07-002, P07-004 | P07-005 | text + html |
| F-MSG-11 | `multipart/mixed` | REQUIRED | P07-002, P08-004 | P07-005, P08-005 | With attachments |
| F-MSG-12 | Deterministic header folding | REQUIRED | P07-001, P07-004 | P07-005 | CRLF, legal folds |
| F-MSG-13 | Address parsing and validation | REQUIRED | P04-002, P07-001 | P04-005, P07-005 | Deterministic errors |
| F-MSG-14 | IDNA domain conversion | REQUIRED | P04-002 | P04-005, P07-005 | ASCII local parts only |
| F-MSG-15 | Non-ASCII display names and bodies | REQUIRED | P04-002, P07-001, P07-002 | P07-005 | Unicode |
| F-MSG-16 | SMTPUTF8 local parts | POST-MVP | P18-002 research only | — | Requires capability negotiation |
| F-MSG-17 | Template rendering | OUT OF SCOPE | — | — | Application concern |

## Attachments

| ID | Feature | Class | Implementation | Tests | Notes |
| --- | --- | --- | --- | --- | --- |
| F-ATT-01 | Buffer attachments | REQUIRED | P08-004 | P08-005, P12-002 | Copied before async |
| F-ATT-02 | Opt-in local file attachments | REQUIRED | P08-001, P08-002 | P08-005 | FS disabled by default |
| F-ATT-03 | Filename encoding | REQUIRED | P07-003 | P07-005, P08-005 | RFC 2231/2047 |
| F-ATT-04 | Explicit or inferred content type | REQUIRED | P07-003 | P07-005, P08-005 | Fixed extension map |
| F-ATT-05 | Per-file and aggregate caps | REQUIRED | P08-003 | P08-005, P13-003 | 10 MiB / 20 MiB |
| F-ATT-06 | Wire-size cap with attachments | REQUIRED | P08-003, P07-004 | P08-005, P13-003 | 30 MiB |
| F-ATT-07 | Safe regular-file-only reads | REQUIRED | P08-002 | P08-005, P11-003 | No symlink/FIFO/device |
| F-ATT-08 | Streaming attachments | POST-MVP | — | — | Conflicts with wire accounting |
| F-ATT-09 | URL/remote attachments | OUT OF SCOPE | — | P11-003 negative test | SSRF surface |
| F-ATT-10 | Path globbing or directory attachments | OUT OF SCOPE | — | — | Unbounded input |

## Delivery

| ID | Feature | Class | Implementation | Tests | Notes |
| --- | --- | --- | --- | --- | --- |
| F-DEL-01 | Single message per call | REQUIRED | P09-002 | P12-003 | No batching |
| F-DEL-02 | Multiple recipients (≤100) | REQUIRED | P06-001, P07-001 | P06-005, P12-002 | Hard cap |
| F-DEL-03 | Structured result | REQUIRED | P02-002, P09-002 | P09-005, P12-003 | No delivery claim |
| F-DEL-04 | Honest recipient-level failure | REQUIRED | P06-003, P09-002 | P06-005, P12-002 | Per ADR-0005 |
| F-DEL-05 | Graceful close | REQUIRED | P09-004, P10-001 | P10-004, P12-003 | Idempotent |
| F-DEL-06 | BUSY on concurrent send | REQUIRED | P04-004, P09-004 | P10-004, P12-003 | No queue |
| F-DEL-07 | Per-recipient independent messages | OUT OF SCOPE | — | P06-005 negative trace | Forbidden |
| F-DEL-08 | Automatic retry | POST-MVP | Future ADR/research decision | — | Not in current implementation scope |
| F-DEL-09 | AbortSignal cancellation | POST-MVP | P18-002 | — | Optional, not required |
| F-DEL-10 | Delivery confirmation / DSN | OUT OF SCOPE | — | — | Requires receiving path |

## Developer experience

| ID | Feature | Class | Implementation | Tests | Notes |
| --- | --- | --- | --- | --- | --- |
| F-DX-01 | TypeScript definitions | REQUIRED | P05-001, P09-001 | P09-005, P12-003 | Strict types |
| F-DX-02 | ESM support | REQUIRED | P14-002 | P14-004 | `import` |
| F-DX-03 | CommonJS support | REQUIRED | P14-002 | P14-004 | `require` |
| F-DX-04 | npm installation without Rust | REQUIRED | P14-004 | P17-004 | Prebuilt binaries |
| F-DX-05 | Stable, clear error messages | REQUIRED | P09-003 | P09-005, P12-003 | Codes + classes |
| F-DX-06 | Runnable examples | REQUIRED | P09-005, P15-001 | P15-005 | Fake relay only |
| F-DX-07 | Documentation set | REQUIRED | P15-001–P15-005 | P15-005 | README, API, errors, troubleshooting |
| F-DX-08 | Strict TypeScript consumer test | REQUIRED | P09-001 | P12-003 | No `any` escape |
| F-DX-09 | Additional transports | POST-MVP | — | — | SMTP-first |
| F-DX-10 | Callback `sendMail` API | OUT OF SCOPE | — | — | Promise-only |
| F-DX-11 | Plugin system | OUT OF SCOPE | — | — | Narrow API |

## Security

| ID | Feature | Class | Implementation | Tests | Notes |
| --- | --- | --- | --- | --- | --- |
| F-SEC-01 | Mandatory TLS verification | REQUIRED | P06-002 | P11-002, P06-005 | No bypass |
| F-SEC-02 | Header and envelope injection prevention | REQUIRED | P04-002, P07-001 | P11-003, P07-005 | CRLF/control chars |
| F-SEC-03 | Credential and content redaction | REQUIRED | P09-003, P11-004 | P11-005, P12-003 | Serialized-error scans |
| F-SEC-04 | Attachment filesystem safety | REQUIRED | P08-001, P08-002 | P11-003, P08-005 | Opt-in, regular files |
| F-SEC-05 | Memory ownership safety | REQUIRED | P05-002, P11-004 | P11-005, P12-002 | Copied buffers |
| F-SEC-06 | Panic containment | REQUIRED | P04-001, P05-004 | P11-005 | No unwind across FFI |
| F-SEC-07 | Dependency and supply-chain review | REQUIRED | P03-002, P11-004 | P11-005, P16-002 | Pinned, inventoried |
| F-SEC-08 | Bounded resource use | REQUIRED | P08-003, P10-003 | P13-003, P11-004 | Caps and cleanup |
| F-SEC-09 | Certificate verification bypass | OUT OF SCOPE | — | P11-002 negative test | Permanent prohibition |

## Distribution and operations

| ID | Feature | Class | Implementation | Tests | Notes |
| --- | --- | --- | --- | --- | --- |
| F-DIS-01 | macOS arm64 package | REQUIRED (if validated) | P14-003 | P14-004 | Drop only by gate decision |
| F-DIS-02 | macOS x64 package | REQUIRED (if validated) | P14-003 | P14-004 | Drop only by gate decision |
| F-DIS-03 | Linux x64 glibc package | REQUIRED (if validated) | P14-003 | P14-004 | Minimum glibc recorded |
| F-DIS-04 | Linux arm64 glibc package | REQUIRED (if validated) | P14-003 | P14-004 | Drop only by gate decision |
| F-DIS-05 | Linux x64 musl package | REQUIRED (if validated) | P14-003 | P14-004 | Alpine compatibility |
| F-DIS-06 | Linux arm64 musl package | REQUIRED (if validated) | P14-003 | P14-004 | Drop only by gate decision |
| F-DIS-07 | Windows x64 MSVC package | REQUIRED (if validated) | P14-003 | P14-004 | MSVC runtime notes |
| F-DIS-08 | Node.js 22 runtime support | REQUIRED (candidate) | P01-004, P14-004 | P14-004 | Validated only |
| F-DIS-09 | Node.js 24 runtime support | REQUIRED (candidate) | P01-004, P14-004 | P14-004 | Validated only |
| F-DIS-10 | Node.js 26 runtime support | OPTIONAL FOR MVP | P01-004, P14-004 | P14-004 | Pre-release risk |
| F-DIS-11 | Provenance and checksums | REQUIRED | P16-002 | P17-001 | Supply chain |
| F-DIS-12 | No postinstall binary download | REQUIRED | P14-002 | P14-004 | Optional packages only |
| F-DIS-13 | WASM fallback | POST-MVP | — | — | Different runtime |
| F-DIS-14 | Node.js runtime support | REQUIRED | P05-003, P14-004 | P14-004 | Exact versions and platform combinations only |
| F-DIS-15 | Bun runtime support | REQUIRED | P05-003, P14-004 | P14-004 | Sendust blocking CI; upstream best-effort is insufficient |
| F-DIS-16 | Deno runtime support | REQUIRED | P05-003, P14-004 | P14-004 | Local node_modules and --allow-ffi; no install scripts |
| F-DIS-17 | Runtime-neutral TypeScript API and isolated native loader | REQUIRED | P03-001, P05-001 | P05-005, P09-005 | Identical semantics across supported runtimes |
| F-DIS-18 | Runtime × OS × CPU × ABI support evidence | REQUIRED | P14-003 | P14-004, P15-001 | Only clean-install and behavioral-tested combinations |
| F-OPS-01 | Explicit lifecycle state | REQUIRED | P04-004, P09-004 | P12-003 | `open/closing/closed` |
| F-OPS-02 | Bounded close drain | REQUIRED | P10-002 | P10-004 | 125 s |
| F-OPS-03 | No GC-dependent shutdown | REQUIRED | P10-001 | P10-004 | Forced-GC tests |
| F-OPS-04 | Structured logs/observability | POST-MVP | — | — | No logging in MVP |
| F-OPS-05 | Queue integration or adapter | HELD | — | — | Not in current roadmap; queue orchestration is caller-owned |

## Post-MVP candidates (not in MVP)

| ID | Candidate | Trigger | Research task |
| --- | --- | --- | --- |
| PM-01 | Connection pooling | Throughput demand | P18-003 |
| PM-02 | Opt-in retry policy | Caller demand with idempotency story | Future planning decision |
| PM-03 | SMTPUTF8 | Internationalized local parts | P18-002 |
| PM-04 | AbortSignal cancellation | Caller demand; must reuse bounded close | P18-002 |
| PM-05 | Streaming attachments | Large-message demand | P18-002 |
| PM-06 | Additional transports (SES, SendGrid, Mailgun) | Provider demand | P18-002 |
| PM-07 | Templates | Application demand | P18-001 |

| PM-09 | Observability hooks | Operational demand | P18-002 |
| PM-10 | `verify()` probe | Diagnostics demand | P18-002 |
| PM-11 | DKIM signing | Deliverability demand | P18-002 |
| PM-12 | Enterprise CA / platform verifier | Enterprise demand | P18-002 |

## Out-of-scope product areas (permanent)

Email marketing, campaigns, contacts, analytics dashboards, open tracking, click tracking, email hosting, web administration, built-in template management, provider-specific REST API integrations, IMAP/POP3 receiving, plaintext SMTP, certificate verification bypass. Internal queue infrastructure and queue adapters are held outside the current roadmap, not designed as post-MVP candidates.

## Requirement coverage map

Each functional and non-functional requirement maps to at least one feature row above. This map is audited by P12-001.

| Requirement | Feature rows |
| --- | --- |
| FR-001 | F-SMTP-01, F-SMTP-04, F-SMTP-05 |
| FR-002 | F-SMTP-04 |
| FR-003 | F-SMTP-05 |
| FR-004 | F-SMTP-02 |
| FR-005 | F-SMTP-03 |
| FR-006 | F-SMTP-06 |
| FR-007 | F-SMTP-07 |
| FR-008 | F-SMTP-08 |
| FR-009 | F-SMTP-09 |
| FR-010 | F-SMTP-11, F-SEC-01 |
| FR-011 | F-SMTP-18 |
| FR-020 | F-MSG-01 |
| FR-021 | F-MSG-02 |
| FR-022 | F-MSG-03 |
| FR-023 | F-MSG-04 |
| FR-024 | F-MSG-05 |
| FR-025 | F-MSG-06 |
| FR-026 | F-MSG-07 |
| FR-027 | F-MSG-08 |
| FR-028 | F-MSG-09 |
| FR-029 | F-MSG-10 |
| FR-030 | F-MSG-11 |
| FR-031 | F-MSG-13, F-MSG-14 |
| FR-032 | F-SEC-02, F-MSG-12 |
| FR-033 | F-MSG-12 |
| FR-034 | F-MSG-09 |
| FR-035 | F-MSG-06, F-MSG-15 |
| FR-040 | F-ATT-01 |
| FR-041 | F-ATT-02 |
| FR-042 | F-ATT-07 |
| FR-043 | F-ATT-03 |
| FR-044 | F-ATT-04 |
| FR-045 | F-ATT-05, F-ATT-06 |
| FR-046 | F-ATT-09, F-ATT-10 |
| FR-050 | F-DEL-01 |
| FR-051 | F-DEL-02 |
| FR-052 | F-DEL-03 |
| FR-053 | F-DEL-04 |
| FR-054 | F-DEL-06 |
| FR-055 | F-DEL-05 |
| FR-056 | F-DEL-05, F-OPS-01 |
| FR-057 | F-DEL-07, F-DEL-08 |
| FR-060 | F-DX-01 |
| FR-061 | F-DX-02 |
| FR-062 | F-DX-03 |
| FR-063 | F-DX-04 |
| FR-064 | F-DX-05 |
| FR-065 | F-DX-06 |
| FR-066 | F-DX-07 |
| FR-067 | F-DX-01, F-DX-08 |
| FR-068 | F-DX-01 |
| NFR-001 | F-SEC-01 |
| NFR-002 | F-SEC-02 |
| NFR-003 | F-SEC-03 |
| NFR-004 | F-SEC-08, F-ATT-05 |
| NFR-005 | (P13 budgets; applies to all rows) |
| NFR-006 | F-DIS-01 through F-DIS-18 |
| NFR-007 | (P12 fixture gate) |
| NFR-008 | F-SEC-03 |
| NFR-009 | F-SEC-06 |
| NFR-010 | F-DX-08 |
| NFR-011 | F-DIS-11 |
| NFR-012 | F-DX-07 |
| NFR-013 | F-SEC-07 |
| NFR-014 | F-SEC-02, F-DX-05 |

## Constraint coverage map

| Constraint | Feature rows / enforcement |
| --- | --- |
| C-01 | All rows; P02-001, P03-001, P04-005, P05-005 |
| C-02 | All rows; call path is fixed |
| C-03 | F-SMTP-01 through F-SMTP-11 |
| C-04 | F-SMTP-01, F-SMTP-06, F-SMTP-07, F-SMTP-08 |
| C-05 | F-DX-01, F-DIS-01 through F-DIS-10 |
| C-06 | F-DIS-08, F-DIS-09, F-DIS-10 |
| C-07 | F-SMTP-04, F-SMTP-05, F-SMTP-11, F-SEC-01 |
| C-08 | F-DEL-06, F-OPS-01 |
| C-09 | F-DEL-05, F-OPS-01, F-OPS-02, F-OPS-03 |
| C-10 | F-SMTP-06, F-SMTP-07, F-SMTP-08, F-OPS-02 |
| C-11 | F-SEC-05, F-ATT-01 |
| C-12 | F-ATT-02, F-ATT-07, F-ATT-09, F-ATT-10 |
| C-13 | F-ATT-05, F-ATT-06, F-DEL-02, F-DEL-06 |
| C-14 | F-MSG-13, F-MSG-14, F-MSG-15, F-MSG-16 |
| C-15 | F-DEL-03, F-DEL-04, F-DEL-08 |
| C-16 | F-SMTP-13, F-SMTP-14, F-OPS-05 |
| C-17 | F-DEL-09 |
| C-18 | F-DEL-04, F-DEL-07 |
| C-19 | F-SEC-03 |
| C-20 | F-DX-05, F-SEC-06 |
| C-21 | F-SEC-07, F-SEC-08 |
| C-22 | F-DX-02, F-DX-03 |
| C-23 | F-DIS-11 |
| C-24 | All rows; enforced by quality gates |
| C-25 | All rows; documentation rule |
| C-26, C-35, C-36 | F-DIS-01 through F-DIS-18 |
| C-27 | F-DX-04, F-DIS-12 |
| C-28 | F-SMTP-12 |
| C-29 | F-SEC-02 |
| C-30 | All out-of-scope rows |
| C-31 | F-DIS-14, F-DIS-15, F-DIS-16 |
| C-32 | C-01, F-DIS-17 |
| C-33 | F-DIS-17 |
| C-34 | F-DIS-16 |
| C-35 | F-DIS-18 |
| C-36 | F-DIS-01 through F-DIS-07, F-DIS-18 |
