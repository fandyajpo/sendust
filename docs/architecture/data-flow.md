# Sendust — Data Flow

This document traces a single `send` call and a `close` call across the four layers. It defines what is copied, when, and which failures are possible at each boundary.

## 1. Send flow

```mermaid
sequenceDiagram
    autonumber
    participant App as Consumer
    participant TS as ts/src
    participant N as sendust-napi
    participant C as sendust-core
    participant L as SMTP client
    participant R as SMTP relay

    App->>TS: transport.send(message)
    TS->>TS: public shape validation
    TS->>N: beginSend(nativeMessage) synchronously
    N->>N: copy Buffers/strings to Rust-owned values and reserve send slot
    TS->>N: executeSend(ownedOperation) asynchronously
    N->>C: core::send(owned message, reserved transport slot)
    C->>C: deterministic validation
    C->>C: resolve file attachments (if opted in)
    C->>C: enforce raw caps
    C->>C: compose MIME
    C->>C: enforce wire cap
    C->>L: open one AsyncSmtpConnection
    L->>R: TCP connect
    L->>R: TLS handshake or STARTTLS
    L->>R: AUTH
    L->>R: MAIL FROM
    loop each recipient
        L->>R: RCPT TO
        R-->>L: exact recipient reply
    end
    L->>R: DATA command
    R-->>L: 354 start mail input
    L->>R: serialized message + terminator
    R-->>L: final response
    L-->>C: transport result or error
    C->>C: map to accepted/rejected/unknown
    C->>L: QUIT best-effort after known final result; drop socket on uncertain DATA
    C-->>N: SendOutcome
    N-->>TS: converted result object
    TS-->>App: SendResult
```

### 1.1 Ownership at each step

| Step | Input ownership | Output ownership | Copied? |
| --- | --- | --- | --- |
| JS call | JS heap | JS heap | — |
| Wrapper validation | JS heap | JS heap | No |
| napi conversion | JS heap | Rust heap | Synchronous owned copy before async scheduling |
| Core validation | Rust heap | Rust heap | No |
| File attachment read | OS file | Rust heap | Yes, bounded |
| MIME composition | Rust heap | Rust heap | Serialized bytes |
| SMTP submission | Rust heap | Sendust session buffer | the client serializes to socket |
| Result conversion | Rust heap | JS heap | Values only, no bodies |

**Invariant:** after the napi conversion returns to the async runtime, no pointer into JS-owned memory is retained. A JS mutation or `ArrayBuffer` detachment after the call cannot alter sent bytes. This is tested in P05-002 and P11-004.

### 1.2 Validation and composition happen before network

All deterministic failures (validation, limits, configuration, MIME composition, wire cap) occur before the first TCP connection. This guarantees:

- A rejected input never opens a socket.
- `BUSY` never dials the relay.
- Tests can assert zero SMTP transactions for invalid input.

### 1.3 SMTP stage certainty

| Stage | If it fails | Result certainty |
| --- | --- | --- |
| DNS/connect/greeting/EHLO/TLS | No message transaction | Connection/TLS error; the outer connect deadline covers DNS, greeting, EHLO and TLS as one boundary |
| AUTH | Authentication failed | `AUTH` error; no message was submitted |
| MAIL FROM | Definitive SMTP refusal | `rejected` outcome |
| RCPT TO | Individual recipient refusal | Record explicit rejected; continue with other recipients; if none accepted, do not issue DATA |
| DATA command | Explicit 4xx/5xx before message body | `rejected` outcome |
| DATA command | Any response other than required 354 (and not a definitive 4xx/5xx) | Protocol error; no message body sent |
| DATA body + terminator + final reply (client `data()`) | Explicit final 4xx/5xx | `rejected` outcome |
| DATA body + terminator + final reply (client `data()`) | Positive final 2xx | `accepted` outcome for RCPT-accepted recipients |
| DATA body + terminator + final reply (client `data()`) | Write/read/reset/timeout or non-final 3xx without final disposition | `unknown` conservatively because the terminator write is not individually observable |

A result is `accepted` only after a positive final response to DATA. An explicit negative final response (4xx/5xx) is a known `rejected` outcome, including when it arrives after the message body. `unknown` is reserved for cases where the final disposition cannot be established, such as losing the connection after the DATA terminator may have reached the relay but before its final reply arrives. Connection, TLS, and AUTH failures are operational errors, not delivery outcomes. Acceptance means relay responsibility, not mailbox delivery.

## 2. Close flow

```mermaid
sequenceDiagram
    autonumber
    participant App as Consumer
    participant TS as ts/src
    participant N as sendust-napi
    participant C as sendust-core
    participant L as SMTP client

    App->>TS: transport.close()
    TS->>N: beginClose() synchronously
    N->>C: state = closing; signal cancellation
    TS->>N: waitClosed() asynchronously
    alt send in flight
        C->>C: cancel at coarse boundary; drop per-send socket
        C-->>N: active send settled
    else idle
        C->>C: no retained SMTP connection; close immediately
    end
    C->>C: state = closed
    C-->>N: close complete or timeout
    N-->>TS: waitClosed Promise resolves or rejects
    TS-->>App: resolved or rejected
```

Rules:

- `close()` is idempotent; repeated calls return the same completion.
- The drain is bounded by the close budget (125 s default).
- `close()` never waits for garbage collection and is never triggered by GC.
- A send started during `closing` fails with `CLOSED` and does not dial.
- A close during a maximal send (120 s) still has 5 s of drain headroom.

## 3. Error path

```text
core error
  ├── createTransport configuration errors -> synchronous throw
  ├── send validation/limit errors -> Promise rejection
  ├── TlsError               -> async rejection
  ├── AuthenticationError    -> async rejection
  ├── ProtocolError          -> async rejection or Unknown outcome
  ├── TimeoutError           -> async rejection
  ├── TransportClosedError   -> async rejection
  ├── TransportBusyError     -> async rejection (no I/O)
  └── InternalError          -> async rejection, generic message
```

The binding maps core categories to stable JS classes and codes. Synchronous preparation failures are wrapped by the TypeScript public method as Promise rejections; no validation error escapes as an unexpected synchronous throw from `send()`.  It strips:

- passwords and auth tokens,
- full message bodies,
- attachment bytes,
- absolute local paths,
- internal Rust panic payloads.

It preserves:

- SMTP numeric response code and server text where safe,
- timeout category and elapsed budget,
- recipient addresses (already caller-supplied),
- host and port.

## 4. Data flow limits

| Point | Limit applied | Failure mode |
| --- | --- | --- |
| Recipient list | 100 total | `LIMIT_RECIPIENTS` before connect |
| Each buffer attachment | 10 MiB | `LIMIT_ATTACHMENT_BYTES` before MIME |
| All raw attachments | 20 MiB | `LIMIT_ATTACHMENT_TOTAL` before MIME |
| File read | per-file cap + 1 probe | `LIMIT_ATTACHMENT_BYTES` |
| Serialized message | 30 MiB | `LIMIT_WIRE_BYTES` before DATA |
| Active sends | 1 | `BUSY` before connect |
| Close drain | 125 s | `TIMEOUT_CLOSE` |

No limit is enforced after bytes have been written to the socket except the total send deadline.

## 5. Backpressure and memory

- The MVP buffers the entire message before sending. There is no streaming.
- Peak memory is approximately: owned message inputs + serialized wire bytes + the client's line/encoding buffer.
- Caps are chosen so worst-case peak remains bounded and documented (see P13-003 for measured numbers).
- Attachment files are read with a bounded read loop; a file that grows during read is detected by the cap probe and rejected.
- No global state, no cross-transport cache, no pool.
