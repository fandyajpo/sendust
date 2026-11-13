# Sendust — System Design

**Status:** proposed; not frozen. P02-005 is the architecture approval gate.  
**Requirements:** [requirements.md](../product/requirements.md).  
**Call path:** `ts/src -> sendust-napi -> sendust-core -> lettre 0.11.23`.

All paths below are **proposed**; no source exists yet.

## 1. Workspace layout (proposed)

```text
sendust/
├── Cargo.toml                      # workspace, shared profile/lints
├── rust-toolchain.toml             # pinned stable toolchain
├── Cargo.lock                      # committed for reproducible builds
├── crates/
│   ├── sendust-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs
│   │       ├── address.rs
│   │       ├── headers.rs
│   │       ├── message.rs
│   │       ├── limits.rs
│   │       ├── mime.rs
│   │       ├── attachment.rs
│   │       ├── transport/
│   │       │   ├── mod.rs
│   │       │   ├── state.rs
│   │       │   ├── smtp.rs
│   │       │   └── timeout.rs
│   │       └── result.rs
│   │   └── tests/                  # Rust integration tests, fake relay harness
│   └── sendust-napi/
│       ├── Cargo.toml
│       ├── build.rs
│       └── src/
│           ├── lib.rs
│           ├── convert.rs
│           ├── error_map.rs
│           └── transport.rs
├── ts/
│   ├── package.json
│   ├── tsconfig.json
│   └── src/
│       ├── index.ts                # public exports
│       ├── transport.ts
│       ├── types.ts
│       ├── errors.ts
│       └── loader.ts               # generated loader integration point
├── tests/                          # JS integration + type tests + clean-consumer fixtures
│   ├── integration/
│   ├── types/
│   └── fixtures/
│       ├── smtp/                   # scripted SMTP server harness
│       └── tls/                    # certificates for negative tests
├── .github/workflows/
├── docs/
└── package.json                    # root package manifest (published)
```

The root `package.json` is the published artifact. `ts/package.json` is the build workspace for the TypeScript wrapper; the release process composes both. The exact split is decided in P14-002 and must not leak workspace paths into the published package.

## 2. `sendust-core`

### 2.1 Public types (Rust-facing contract)

```rust
pub struct TransportOptions {
    pub relay: RelayOptions,
    pub timeouts: TimeoutOptions,
    pub attachments: AttachmentPolicy,
    pub limits: Limits,
}

pub struct RelayOptions {
    pub host: String,
    pub port: u16,
    pub security: SecurityMode,     // ImplicitTls | StartTls
    pub ehlo_name: Option<String>,
    pub auth: Option<AuthOptions>,  // Plain | Login
}

pub struct TimeoutOptions {
    pub connect: Duration,          // default 10s
    pub operation: Duration,        // default 30s
    pub total: Duration,            // default 120s
    pub close: Duration,            // default 125s
}

pub struct Limits {
    pub max_recipients: usize,      // 100
    pub max_attachment_bytes: u64,  // 10 MiB
    pub max_attachments_total_bytes: u64, // 20 MiB
    pub max_wire_bytes: u64,        // 30 MiB
}

pub struct OutgoingMessage {
    pub from: Mailbox,
    pub to: Vec<Mailbox>, // may be empty when cc or bcc supplies recipients
    pub cc: Vec<Mailbox>,
    pub bcc: Vec<Mailbox>,
    pub reply_to: Vec<Mailbox>,
    pub subject: String,
    pub text: Option<String>,
    pub html: Option<String>,
    pub headers: Vec<CustomHeader>,
    pub attachments: Vec<AttachmentInput>,
}

pub enum AttachmentInput {
    Buffer { filename: String, content_type: Option<String>, data: Vec<u8> },
    File { filename: Option<String>, content_type: Option<String>, path: PathBuf },
}

pub enum SendOutcome {
    Accepted { recipients: Vec<RecipientOutcome>, response: Option<SmtpResponse> },
    Rejected { recipients: Vec<RecipientOutcome>, response: Option<SmtpResponse> },
    Unknown { recipients: Vec<RecipientOutcome>, stage: AmbiguityStage },
}

pub struct RecipientOutcome {
    pub address: String,
    pub status: RecipientStatus,   // Accepted | Rejected | Attempted | Unknown
}
```

The exact shape is proposed and must not be described as frozen until P02-005 is approved. The important property is that `SendOutcome` cannot represent mailbox delivery.

### 2.2 Validation order (deterministic)

Validation runs in this order and stops at the first failure, so error precedence is stable:

1. Transport options: relay host/port, security mode, auth mode, timeout finiteness.
2. Message identity: From present, at least one To/CC/BCC.
3. Recipient count and address parsing (ASCII local part, IDNA domain).
4. Header names/values and CRLF/control-character rejection.
5. Subject and body encoding feasibility.
6. Attachment metadata: count, filename, explicit content type validity.
7. Attachment raw bytes caps (buffer attachments first, file attachments after safe read).
8. MIME composition and wire-size cap.
9. Network preconditions (host resolves, config coherent).

A failure at any step returns one error with a stable code. No partial send begins before all validation and composition complete.

### 2.3 Message and MIME model

- `Mailbox` holds an ASCII local part, an IDNA-normalized domain, and an optional Unicode display name.
- Display names and subjects are RFC 2047 encoded when non-ASCII.
- Bodies are encoded with quoted-printable or base64 according to content and size policy; the policy is deterministic and tested.
- Structure:
  - text only → `text/plain`
  - html only → `text/html`
  - both → `multipart/alternative`
  - attachments present → `multipart/mixed` wrapping the body part
- BCC recipients are passed to the SMTP envelope but never written to headers.
- Line endings are CRLF; headers are folded legally.
- Wire-size accounting is performed on the final serialized bytes before DATA.

### 2.4 Transport state machine

```text
        close()                drain complete
  open ─────────► closing ─────────────────► closed
    │                │
    │ send()         │ send() -> CLOSED
    ▼                ▼
  sending          (no new send)
    │
    └── outcome -> open
```

Rules:

- One `send` in flight. A second `send` returns `BUSY` immediately without dialing.
- `close()` is idempotent. Concurrent `close()` calls share the same drain.
- `close()` during connect/auth/DATA requests cancellation of the in-flight operation and waits up to the close budget.
- Cancellation before DATA acceptance is possible may end as an operational cancellation/timeout error. If the DATA terminator may have reached the relay and no final reply is observed, the send outcome is `Unknown`; close cannot turn this into a definite rejection.
- The close budget (125 s) is greater than the total send budget (120 s) so a close during a maximal send can still drain.
- After `closed`, every `send` returns `CLOSED`.
- No finalizer, GC hook, or process-exit handler is required for correctness.

### 2.5 Timeout model

A single absolute deadline is computed per send:

```text
deadline = start + min(total, remaining close budget)
```

- `connect` bounds TCP + TLS/STARTTLS establishment.
- `operation` bounds each SMTP command/response wait.
- `total` bounds the entire send, including composition wait and DATA.
- A stage timer may fire earlier; the reported error names the category and elapsed budget. Exact per-stage attribution is limited to what P01-003 proved.
- All durations must be finite and > 0; `0`, negative, `NaN`, and infinity are rejected.

### 2.6 SMTP orchestration

- Core builds a lettre async SMTP transport per send (no pool).
- TLS is configured with rustls `ring` + `webpki-roots` per P01-002. Verification is never configurable off.
- Auth is PLAIN or LOGIN; unsupported mechanisms are rejected at configuration time.
- The envelope is submitted as a whole. Recipient policy follows ADR-0005:
  - If per-recipient RCPT status is observable through the chosen API, report it honestly.
  - Otherwise reject the whole envelope before DATA when any recipient is rejected, and report `unknown`/`attempted` where certainty is unavailable.
  - Never send one message per recipient.
- A positive final DATA reply (2xx) → `Accepted`.
- An explicit negative final DATA reply (4xx/5xx) → `Rejected`; this is a known refusal, not an ambiguous result.
- Missing final reply after the DATA terminator may have reached the relay → `Unknown`; no retry.
- Connection, TLS, and AUTH failures before message submission are operational errors, not `Rejected` delivery outcomes.

### 2.7 Attachment policy

- `AttachmentPolicy { filesystem_enabled: bool }` defaults to `false`.
- With filesystem disabled, any `File` attachment fails validation before any filesystem access.
- With filesystem enabled:
  - Open the path with no-follow semantics where the platform supports it.
  - `fstat` the opened descriptor; require a regular file.
  - Reject symlink, directory, FIFO, device, socket, and unreadable file.
  - Read at most the per-file cap + 1 byte to detect overflow.
  - Never echo the absolute path in public errors; use the caller-supplied filename or a redacted marker.
- Buffer attachments are copied before async work.
- Aggregate raw attachment bytes are checked before MIME composition; wire size is checked after composition.

## 3. `sendust-napi`

### 3.1 Exports

| Export | Kind | Purpose |
| --- | --- | --- |
| `NativeTransport` | class | Owns core transport and lifecycle |
| `NativeTransport.create(options)` | sync | Validate shape, construct core transport |
| `NativeTransport.send(message)` | async | Copy inputs, run core send |
| `NativeTransport.close()` | async | Idempotent bounded drain |
| `NativeTransport.state` | getter | `open` / `closing` / `closed` |
| `nativeVersion()` | sync | Diagnostic version string |

Names and casing follow napi-rs conventions; the TypeScript wrapper re-exports the stable public names.

### 3.2 Conversion rules

- `Buffer`/`ArrayBuffer`/typed arrays → `Vec<u8>` copy before async.
- Strings → `String` with UTF-8 validation; invalid UTF-16 is rejected, not lossily replaced.
- Numbers → bounded integer conversion with range checks; no silent truncation.
- Options objects → core structs with unknown fields rejected or ignored per the frozen contract (decided in P02-002; default: reject unknown keys to prevent typos).
- Errors → JS `Error` subclass with `code`, `category`, `message`, optional `details`; credentials and content removed.
- Panics → caught at the exported boundary where unwind is enabled; converted to an internal error. `panic = "abort"` is not used for the addon.

### 3.3 Async model

- `#[napi] pub async fn` runs on the napi-managed Tokio 1 runtime.
- The binding never creates a runtime.
- The transport is retained for the lifetime of in-flight work via explicit ownership, not GC reachability.
- Blocking file reads run on a blocking pool (`tokio::task::spawn_blocking` or equivalent), never on the reactor thread.
- `close()` cancels/awaits the in-flight send within the close budget.

## 4. `ts/src`

### 4.1 Public API (frozen in P09-001)

```ts
export interface TransportOptions {
  host: string;
  port: number;
  secure: "tls" | "starttls";
  auth?: { user: string; pass: string; method?: "plain" | "login" };
  ehloName?: string;
  timeouts?: { connectMs?: number; operationMs?: number; totalMs?: number; closeMs?: number };
  attachments?: { allowFileAccess?: boolean };
  limits?: { maxRecipients?: number; maxAttachmentBytes?: number; maxTotalAttachmentBytes?: number; maxWireBytes?: number };
}

export interface Message {
  from: string | { name?: string; address: string };
  to: Array<string | { name?: string; address: string }>;
  cc?: Array<string | { name?: string; address: string }>;
  bcc?: Array<string | { name?: string; address: string }>;
  replyTo?: Array<string | { name?: string; address: string }>;
  subject: string;
  text?: string;
  html?: string;
  headers?: Record<string, string>;
  attachments?: Attachment[];
}

export type Attachment =
  | { filename: string; content: Buffer | Uint8Array; contentType?: string }
  | { path: string; filename?: string; contentType?: string };

export interface SendResult {
  status: "accepted" | "rejected" | "unknown";
  recipients: Array<{ address: string; status: "accepted" | "rejected" | "attempted" | "unknown" }>;
  response?: { code: number; message: string };
}

export interface Transport {
  send(message: Message): Promise<SendResult>;
  close(): Promise<void>;
  readonly state: "open" | "closing" | "closed";
}

export function createTransport(options: TransportOptions): Transport;
```

The wrapper adds no behavior beyond shape validation, default documentation, and error class identity.

### 4.2 Module formats

- ESM entry: `dist/index.mjs` (or `index.js` with `"type": "module"` per final packaging decision).
- CommonJS entry: `dist/index.cjs`.
- Types: `dist/index.d.ts`.
- `exports` map declares `import`, `require`, and `types`.
- Both formats load the same native addon through the generated loader.

The exact layout is validated in P14-002/P14-004. No dual-package hazard: a single shared implementation module is wrapped by thin format-specific entry files.

### 4.3 Error classes

```ts
export class SendustError extends Error {
  readonly code: string;
  readonly category: string;
  readonly details?: Record<string, unknown>;
}
export class ValidationError extends SendustError {}
export class ConfigurationError extends SendustError {}
export class TlsError extends SendustError {}
export class AuthenticationError extends SendustError {}
export class ProtocolError extends SendustError {}
export class TimeoutError extends SendustError {}
export class TransportClosedError extends SendustError {}
export class TransportBusyError extends SendustError {}
export class LimitError extends SendustError {}
```

Codes are stable strings; classes are stable for `instanceof`. The full catalog is in [error-model.md](error-model.md).

## 5. Test seams

- **Fake SMTP server**: scripted dialogue, deterministic responses per stage, no sleeps beyond injected delays. Runs on loopback ephemeral ports.
- **Fake TLS**: generated fixtures for self-signed, expired, wrong host, incomplete chain, malformed certificate.
- **Fault injection**: accept/reject per RCPT, fail after DATA, reset at each stage, stall at each stage.
- **Clock control**: core timeouts take an injectable clock/timer trait so tests advance time without real waits.
- **Filesystem fixtures**: regular file, symlink, directory, FIFO, device where available, missing path, unreadable file.
- **JS boundary fixtures**: buffer mutation, detachment, forced GC, process exit.

## 6. Dependency choices (proposed)

| Dependency | Purpose | MVP | Alternative | Notes |
| --- | --- | --- | --- | --- |
| `lettre` 0.11.23 | SMTP, TLS, auth, message builder | Required | Custom SMTP client; `mail-send` | Mature, async, Tokio support. Pin exact version. |
| `tokio` 1 | Async runtime via napi | Required | async-std | napi-rs manages the runtime; do not create another. |
| `rustls` 0.23 | TLS | Required | native-tls | `ring` provider, `webpki-roots`; no verification bypass. |
| `napi` / `napi-derive` 3 | Node-API binding | Required | neon, node-bindgen | Current docs target v3; verify Node-API floor. |
| `serde` / `serde_json` | Optional structured details | Optional | manual | Only if error/result details need serialization. |
| `thiserror` | Core error ergonomics | Optional | manual impls | Does not cross FFI; mapping stays explicit. |
| `tracing` | Internal diagnostics | Optional | `log` | No message content or credentials in spans. |
| `mail-parser` | MIME test verification | Test-only | hand-rolled assertions | Use in tests to parse emitted MIME; not a runtime dependency. |
| `criterion` | Benchmarks | Phase 13 | built-in bench | Dev-only. |

Every dependency requires license, maintenance, and security review at P03-002/P11-004. Exact versions are pinned in `Cargo.lock` and recorded in the dependency inventory (P16-002).

## 7. Explicit non-decisions

The following are intentionally unresolved until their owning task:

- Exact per-stage timeout attribution (P01-003).
- Recipient partial-status observability (P01-003/P06-003, ADR-0005).
- Node-API level if a required feature needs more than 8 (P01-004).
- Final platform matrix (P14-003).
- Performance budgets (P13-001).

Each has a validation task and cannot be assumed in implementation.
