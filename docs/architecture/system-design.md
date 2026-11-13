# Sendust — System Design

**Status:** approved design baseline at P02-005; implementation and runtime validation remain gated downstream.  
**Requirements:** [requirements.md](../product/requirements.md).  
**Call path:** `ts/src -> sendust-napi -> sendust-core -> lettre 0.11.23`.

All paths below describe the approved design baseline; application source has not yet been implemented.

## 1. Workspace layout (approved design)

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

pub enum RecipientStatus { Accepted, Rejected, Unknown }

pub struct RecipientOutcome {
    pub address: String,
    pub status: RecipientStatus,
}
```

The exact Rust names are implementation details, but the approved semantics are fixed by P02-005. `SendOutcome` cannot represent mailbox delivery.

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
- `close()` atomically transitions to `closing`, signals cancellation, and waits up to the close budget. Before message transmission begins, cancellation returns a closed/cancelled error or a definitive SMTP rejection. Once `message()` begins, cancellation or transport failure is conservatively `Unknown` unless an explicit final negative SMTP reply is received. Drop the connection without issuing QUIT while DATA state is uncertain.
- The close budget (125 s) is greater than the total send budget (120 s) so a close during a maximal send can still drain.
- After `closed`, every `send` returns `CLOSED`.
- No finalizer, GC hook, or process-exit handler is required for correctness.

### 2.5 Timeout model

A single absolute deadline is computed per send:

```text
deadline = start + min(total, remaining close budget)
```

- `connect` bounds DNS + TCP + greeting + EHLO + implicit TLS/required STARTTLS negotiation through one outer deadline. lettre's inner timeout only bounds TCP attempts; Sendust must wrap the complete connection future.
- `operation` bounds each SMTP command/response wait.
- `total` bounds the entire send, including composition wait and DATA.
- An operation timer wraps each public lettre operation. Report only coarse boundaries: connection setup; AUTH; MAIL; each RCPT; DATA command; DATA body+terminator+final response; cleanup. Do not claim internal socket-read/write attribution.
- Defaults are hard maximums: connect ≤10 s, operation ≤30 s, total ≤120 s, close ≤125 s. Values may be lowered but not raised. Each is an integer of at least 1 s; connect and operation ≤ total; close ≥ total. Zero, negative, sub-second, NaN, infinity, and inconsistent combinations are rejected.

### 2.6 SMTP orchestration

- Core creates one `AsyncSmtpConnection` per send (no pool), using lettre's public lower-level async connection API. The selected API is required for individual RCPT responses; Sendust does not implement SMTP parsing or command formatting.
- lettre features are pinned with `default-features = false` and `smtp-transport`, `tokio1-rustls`, `ring`, and `webpki-roots`; this excludes lettre's default pool/native-tls features. Cargo compilation is a P03-002 hard gate. Build `TlsParameters` with `CertificateStore::WebpkiRoots` and TLS 1.2 minimum. Never call dangerous certificate/hostname bypass methods. Implicit TLS uses `connect_tokio1` with TLS parameters; STARTTLS connects without TLS then requires `can_starttls()` and `starttls()` before AUTH. No plaintext fallback.
- Auth uses lettre `Credentials` and only `Mechanism::Plain` / `Mechanism::Login`, after TLS. Validate the returned AUTH response is 235; do not treat arbitrary 2xx/3xx as authenticated.
- Send one `MAIL FROM`, then one `RCPT TO` command per recipient through `AsyncSmtpConnection::command`. Record exact 2xx acceptance and 4xx/5xx rejection using `Error::status()`; check response classes explicitly because lettre treats both 2xx and 3xx as generic positive responses. If no recipient is accepted, do not issue DATA. Otherwise issue exactly one DATA transaction for the accepted-recipient envelope. This is one message/one SMTP transaction, not per-recipient message fan-out.
- Require DATA command reply 354 before sending body bytes. A positive final DATA reply (2xx) → overall `accepted`; RCPT-accepted recipients become `accepted`, while explicit RCPT refusals remain `rejected`.
- An explicit negative final DATA reply (4xx/5xx) → overall `rejected`; RCPT-accepted recipients also become `rejected`.
- A non-final/unexpected 3xx after message transmission, or a transport/write/read error once `message()` begins, yields `unknown` because lettre does not expose a public checkpoint proving the final disposition. No retry.
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
| `NativeTransport.beginSend(message)` | internal sync | Validate/copy inputs, reserve slot, create opaque owned operation |
| `NativeTransport.executeSend(operation)` | internal async | Run core send on managed Tokio; returns Promise |
| `NativeTransport.beginClose()` | internal sync | Atomically enter closing and signal cancellation |
| `NativeTransport.waitClosed()` | internal async | Await bounded drain; returns Promise |
| `NativeTransport.state` | getter | `open` / `closing` / `closed` |
| `nativeVersion()` | sync | Diagnostic version string |

Names and casing follow napi-rs conventions; the TypeScript wrapper re-exports the stable public names.

### 3.2 Conversion rules

- `Buffer`/`ArrayBuffer`/typed arrays are synchronously snapshotted into Rust-owned `Vec<u8>` before async work is scheduled. Do not retain `BufferSlice`, raw N-API pointers, or JS-backed views across an await.
- Strings → `String` with UTF-8 validation; invalid UTF-16 is rejected, not lossily replaced.
- Numbers → bounded integer conversion with range checks; no silent truncation.
- Options objects → core structs; unknown keys are rejected with `VALIDATION_INVALID_OPTION`.
- Errors → JS `Error` subclass with `code`, `category`, `message`, optional `details`; credentials and content removed.
- Panics → caught at the exported boundary where unwind is enabled; converted to an internal error. `panic = "abort"` is not used for the addon.

### 3.3 Async model

- N-API synchronous entrypoints snapshot JS values and reserve the send slot before scheduling async work; an internal opaque operation owns the Rust data. The async execution method returns a Promise and runs on napi-managed Tokio 1. The TypeScript wrapper exposes only `send()`/`close()`. `send()` calls `beginSend()` then `executeSend()`; `close()` calls `beginClose()` then `waitClosed()`. Synchronous preparation failures become Promise rejections.
- The binding never creates a runtime.
- The transport is retained for the lifetime of in-flight work via explicit ownership, not GC reachability.
- Blocking file reads run on a blocking pool (`tokio::task::spawn_blocking` or equivalent), never on the reactor thread.
- `beginClose()` synchronously transitions state and signals cancellation; `waitClosed()` returns a Promise that awaits bounded cleanup. The public TypeScript `close()` composes these internal methods.

## 4. `ts/src`

### 4.1 Public API design (approved at P02-005; implementation gate P09-005)

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
  to?: Array<string | { name?: string; address: string }>;
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
  recipients: Array<{ address: string; status: "accepted" | "rejected" | "unknown" }>;
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

## 6. Dependency choices (approved design baseline; packaging details remain phase-gated)

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
