/**
 * Public Sendust contract types.
 *
 * These mirror `docs/architecture/public-api.md`. Sendust reports **relay
 * acceptance, never delivery**: `accepted` means the SMTP relay took
 * responsibility for the message, not that a mailbox received it.
 *
 * A transport owns a bounded queue and a pool of native sessions. Concurrent
 * `send()` calls are accepted and scheduled automatically, one message per
 * session at a time, and only transient failures are retried — never `unknown`.
 *
 * Timeout defaults are connect 10 s, per-operation 30 s, total 120 s, close/drain
 * 125 s. Each may be lowered but never raised. Attachment limits default to 100
 * recipients, 10 MiB per attachment, 20 MiB aggregate raw attachments, and
 * 30 MiB serialized wire message; limits may be lowered but never raised.
 */

/** Overall result of one send. Never a delivery claim. */
export type DeliveryStatus = "accepted" | "rejected" | "unknown";

/** Outcome of one recipient. `accepted` requires a positive final DATA reply. */
export type RecipientStatus = "accepted" | "rejected" | "unknown";

/** The relay's final SMTP response, when one was observed. */
export interface SmtpResponse {
  readonly code: number;
  readonly message: string;
}

/** One recipient and the status the relay gave it. */
export interface RecipientOutcome {
  readonly address: string;
  readonly status: RecipientStatus;
}

/**
 * Result of `send()`.
 *
 * - `accepted`: the relay accepted responsibility for the message.
 * - `rejected`: the relay explicitly refused it; nothing is retried.
 * - `unknown`: the final disposition could not be established after DATA may
 *   have reached the relay; do not retry automatically.
 */
export interface SendResult {
  readonly status: DeliveryStatus;
  readonly recipients: RecipientOutcome[];
  readonly response?: SmtpResponse;
}

/**
 * Relay credentials. `method` forces one AUTH mechanism; omit it to let the
 * engine prefer PLAIN and fall back to LOGIN from the relay's EHLO list. An
 * unadvertised forced mechanism fails the send rather than falling back.
 */
export interface AuthOptions {
  readonly user: string;
  readonly pass: string;
  readonly method?: "plain" | "login";
}

/**
 * Timeout budgets in milliseconds. Defaults are also the maxima.
 *
 * | Field | Default | Range |
 * | --- | --- | --- |
 * | `connectMs` | 10000 | 1000–10000, and ≤ `totalMs` |
 * | `operationMs` | 30000 | 1000–30000, and ≤ `totalMs` |
 * | `totalMs` | 120000 | 1000–120000, and ≥ `connectMs` and `operationMs` |
 * | `closeMs` | 125000 | `totalMs`–125000 |
 */
export interface TimeoutOptions {
  readonly connectMs?: number;
  readonly operationMs?: number;
  readonly totalMs?: number;
  readonly closeMs?: number;
}

/**
 * Hard resource caps. Defaults are also the maxima; values may be lowered.
 *
 * `maxAttachmentBytes` is 10 MiB (10485760), `maxTotalAttachmentBytes` 20 MiB
 * (20971520), and `maxWireBytes` 30 MiB (31457280) by default.
 */
export interface LimitOptions {
  readonly maxRecipients?: number;
  readonly maxAttachmentBytes?: number;
  readonly maxTotalAttachmentBytes?: number;
  readonly maxWireBytes?: number;
}

/**
 * Transport configuration. `secure` has no plaintext mode, and there is no
 * option to disable TLS verification.
 */
export interface TransportOptions {
  /** Relay hostname; non-empty, no whitespace or control characters. */
  readonly host: string;
  /** Relay port; integer 1–65535. */
  readonly port: number;
  /** `tls` for implicit TLS (usually 465) or `starttls` (usually 587). */
  readonly secure: "tls" | "starttls";
  readonly auth?: AuthOptions;
  /** EHLO name; defaults to `localhost`. */
  readonly ehloName?: string;
  readonly timeouts?: TimeoutOptions;
  readonly limits?: LimitOptions;
  /**
   * Optional PEM trust anchor added on top of the bundled WebPKI roots.
   *
   * Verification is never disabled: an unknown relay still fails closed, and a
   * malformed anchor is rejected as `VALIDATION_INVALID_OPTION`.
   */
  readonly tlsRootCertificate?: string;
  /** Filesystem access for path attachments; disabled unless explicitly enabled. */
  readonly attachments?: { readonly allowFileAccess?: boolean };
  /**
   * Automatic delivery management. Every field is an optional override: the
   * defaults accept bursts, pool sessions, bound the queue, and retry only
   * transient failures, so a normal caller sets nothing here.
   */
  readonly delivery?: DeliveryOptions;
}

/** A mailbox given as a bare address or with a display name. */
export interface MailboxLike {
  readonly name?: string;
  readonly address: string;
}

/** Envelope address: a string, or an object with an optional display name. */
export type Address = string | MailboxLike;

/** One custom header. Reserved names cannot be overridden. */
export interface HeaderLike {
  readonly name: string;
  readonly value: string;
}

/** An in-memory attachment; bytes are copied before the send returns. */
export interface BufferAttachment {
  readonly filename: string;
  readonly content: Uint8Array;
  readonly contentType?: string;
}

/** A local regular file; requires `attachments.allowFileAccess: true`. */
export interface FileAttachment {
  readonly path: string;
  readonly filename?: string;
  readonly contentType?: string;
}

/**
 * One attachment. There is no URL, stream, or remote form.
 *
 * Without `contentType`, the MIME type is inferred from the filename extension
 * or falls back to `application/octet-stream`.
 */
export type Attachment = BufferAttachment | FileAttachment;

/**
 * One outgoing message. A message needs at least one recipient across `to`,
 * `cc`, and `bcc`, and at least one of `text` or `html`.
 */
export interface Message {
  readonly from: Address;
  readonly to?: Address[];
  readonly cc?: Address[];
  readonly bcc?: Address[];
  readonly replyTo?: Address[];
  readonly subject: string;
  readonly text?: string;
  readonly html?: string;
  readonly headers?: HeaderLike[] | Record<string, string>;
  readonly attachments?: Attachment[];
}

/** Transport lifecycle state, observed synchronously. */
export type TransportState = "open" | "closing" | "closed";

/**
 * Automatic delivery management. Defaults are safe for normal workloads and
 * need no tuning; every value is bounded, and an out-of-range value is rejected
 * with `VALIDATION_INVALID_OPTION` before any message is accepted.
 *
 * | Field | Default | Range |
 * | --- | --- | --- |
 * | `maxConcurrent` | 8 | 1–32 |
 * | `maxConnections` | `maxConcurrent` | 1–32 |
 * | `maxQueue` | 10000 | 1–1000000 |
 * | `maxQueueBytes` | 67108864 (64 MiB) | 1–1073741824 |
 * | `messagesPerSecond` | 0 (unlimited) | 0–10000 |
 * | `maxAttempts` | 3 | 1–10 |
 * | `retryBaseMs` | 250 | 0–60000 |
 * | `retryMaxMs` | 30000 | 0–600000, and ≥ `retryBaseMs` |
 * | `maxMessagesPerSession` | 100 | 1–10000 |
 * | `sessionIdleMs` | 30000 | 0–600000 |
 */
export interface DeliveryOptions {
  /** Sends allowed at once; concurrency grows toward it and backs off from it. */
  readonly maxConcurrent?: number;
  /** Native sessions the pool may create. Each runs one send at a time. */
  readonly maxConnections?: number;
  /** Messages waiting for a session; beyond this, `send()` rejects. */
  readonly maxQueue?: number;
  /** Bytes waiting for a session, estimated from text, HTML, and buffers. */
  readonly maxQueueBytes?: number;
  /** Token-bucket rate across all sessions; 0 disables the limiter. */
  readonly messagesPerSecond?: number;
  /** Total attempts for a retryable failure, including the first one. */
  readonly maxAttempts?: number;
  /** First backoff window; each further attempt doubles it. */
  readonly retryBaseMs?: number;
  /** Backoff cap. */
  readonly retryMaxMs?: number;
  /** Messages one authenticated SMTP session may serve before it is closed. */
  readonly maxMessagesPerSession?: number;
  /** How long an idle session may be reused before a fresh connection. */
  readonly sessionIdleMs?: number;
}

/** Observable delivery-engine state. Counters are monotonic per transport. */
export interface TransportStats {
  readonly state: TransportState;
  readonly queueDepth: number;
  readonly queueBytes: number;
  readonly active: number;
  readonly connections: number;
  readonly concurrency: number;
  readonly accepted: number;
  readonly rejected: number;
  readonly unknown: number;
  readonly retried: number;
  readonly queueRejections: number;
}
