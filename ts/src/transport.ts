/**
 * Runtime-neutral transport facade.
 *
 * Owns lifecycle composition over the native binding and the automatic delivery
 * engine only. SMTP, MIME, validation, and delivery policy stay in
 * `sendust-core`; queueing, pooling, classification, and retry policy live in
 * `policy.ts` and `engine.ts`.
 */

import { DeliveryEngine, type EngineOptions } from "./engine.ts";
import { SendustClosedError, SendustError, SendustValidationError, fromNative } from "./errors.ts";
import { DEFAULT_LIMITS, assertAttachmentBudget } from "./limits.ts";
import { loadNative, type NativeTransport } from "./loader.ts";
import type {
  Address,
  Attachment,
  AuthOptions,
  DeliveryOptions,
  Message,
  SendResult,
  TransportOptions,
  TransportState,
  TransportStats,
} from "./types.ts";

const TRANSPORT_KEYS = new Set([
  "host",
  "port",
  "secure",
  "auth",
  "ehloName",
  "timeouts",
  "limits",
  "attachments",
  "tlsRootCertificate",
  "delivery",
]);
const AUTH_KEYS = new Set(["user", "pass", "accessToken", "method"]);
const TIMEOUT_KEYS = new Set(["connectMs", "operationMs", "totalMs", "closeMs"]);
const LIMIT_KEYS = new Set([
  "maxRecipients",
  "maxAttachmentBytes",
  "maxTotalAttachmentBytes",
  "maxWireBytes",
]);
const ATTACHMENT_KEYS = new Set(["allowFileAccess"]);
const DELIVERY_KEYS = new Set([
  "maxConcurrent",
  "maxConnections",
  "maxQueue",
  "maxQueueBytes",
  "messagesPerSecond",
  "maxAttempts",
  "retryBaseMs",
  "retryMaxMs",
  "maxMessagesPerSession",
  "sessionIdleMs",
]);

/** Safe defaults: no tuning is required for a normal workload. */
const DELIVERY_DEFAULTS = {
  maxConcurrent: 8,
  maxQueue: 10_000,
  maxQueueBytes: 64 << 20,
  messagesPerSecond: 0,
  maxAttempts: 3,
  retryBaseMs: 250,
  retryMaxMs: 30_000,
  maxMessagesPerSession: 100,
  sessionIdleMs: 30_000,
};

/** The core's default close budget, used when the caller configures none. */
const CLOSE_MS_DEFAULT = 125_000;
/** The core's default whole-send budget, reused as the retry budget. */
const TOTAL_MS_DEFAULT = 120_000;

function closedError(): SendustClosedError {
  return new SendustClosedError({
    code: "TRANSPORT_CLOSED",
    category: "closed",
    message: "transport is closed",
  });
}

function invalidOption(): SendustValidationError {
  return new SendustValidationError({
    code: "VALIDATION_INVALID_OPTION",
    category: "validation",
    message: "invalid transport option",
  });
}

/** Rejects unknown keys so a typo fails instead of being silently ignored. */
function assertKnownKeys(value: object, allowed: Set<string>): void {
  for (const key of Object.keys(value)) {
    if (!allowed.has(key)) {
      throw invalidOption();
    }
  }
}

/** One bearer token, validated as a non-empty string without control characters. */
function resolveToken(value: string | (() => string) | undefined): string {
  const resolved = typeof value === "function" ? value() : value;
  if (
    typeof resolved !== "string" ||
    resolved.length === 0 ||
    /[\u0000-\u001f\u007f]/.test(resolved)
  ) {
    throw invalidOption();
  }
  return resolved;
}

/**
 * The `auth` record the native layer takes.
 *
 * Exactly one secret is required: `pass` for PLAIN/LOGIN or `accessToken` for
 * XOAUTH2 (ADR-0023). A token function is stored as-is and resolved once per
 * session, so a long-lived transport can hand each new session a fresh token.
 */
function nativeAuth(auth: AuthOptions): Record<string, unknown> {
  const hasPass = auth.pass !== undefined;
  const hasToken = auth.accessToken !== undefined;
  if (hasPass === hasToken) {
    // Neither leaves nothing to authenticate with; both make the intended
    // mechanism ambiguous.
    throw invalidOption();
  }
  if (auth.method === "xoauth2" && !hasToken) {
    throw invalidOption();
  }
  if (auth.method !== undefined && auth.method !== "xoauth2" && !hasPass) {
    throw invalidOption();
  }

  if (hasToken) {
    return {
      user: auth.user,
      accessToken:
        typeof auth.accessToken === "function" ? auth.accessToken : resolveToken(auth.accessToken),
      method: "xoauth2",
    };
  }
  return { user: auth.user, pass: auth.pass, method: auth.method };
}

/**
 * Resolves a per-session bearer token immediately before a native transport is
 * built: once for the first session and once per pool growth.
 */
function withFreshToken(
  native: Record<string, unknown>,
  options: TransportOptions,
): Record<string, unknown> {
  const auth = options.auth;
  if (!auth || auth.accessToken === undefined) {
    return native;
  }
  return {
    ...native,
    auth: {
      ...(native.auth as Record<string, unknown>),
      accessToken: resolveToken(auth.accessToken),
    },
  };
}

/** One delivery option, clamped to its documented range. */
function bounded(
  name: keyof DeliveryOptions,
  value: number | undefined,
  min: number,
  max: number,
  fallback: number,
): number {
  if (value === undefined) {
    return fallback;
  }
  if (!Number.isInteger(value) || value < min || value > max) {
    throw new SendustValidationError({
      code: "VALIDATION_INVALID_OPTION",
      category: "validation",
      message: `invalid delivery option: ${name}`,
    });
  }
  return value;
}

/** Validates `delivery` and turns it into engine options. */
function deliverySettings(
  delivery: DeliveryOptions | undefined,
  closeMs: number,
  totalMs: number,
): Omit<EngineOptions, "createTransport" | "first" | "sessionIdleMs"> {
  if (delivery) {
    assertKnownKeys(delivery, DELIVERY_KEYS);
  }

  const maxConcurrent = bounded(
    "maxConcurrent",
    delivery?.maxConcurrent,
    1,
    32,
    DELIVERY_DEFAULTS.maxConcurrent,
  );
  const retryBaseMs = bounded(
    "retryBaseMs",
    delivery?.retryBaseMs,
    0,
    60_000,
    DELIVERY_DEFAULTS.retryBaseMs,
  );
  const retryMaxMs = bounded(
    "retryMaxMs",
    delivery?.retryMaxMs,
    0,
    600_000,
    DELIVERY_DEFAULTS.retryMaxMs,
  );
  if (retryMaxMs < retryBaseMs) {
    throw invalidOption();
  }

  return {
    maxConcurrent,
    maxConnections: bounded("maxConnections", delivery?.maxConnections, 1, 32, maxConcurrent),
    maxQueue: bounded("maxQueue", delivery?.maxQueue, 1, 1_000_000, DELIVERY_DEFAULTS.maxQueue),
    maxQueueBytes: bounded(
      "maxQueueBytes",
      delivery?.maxQueueBytes,
      1,
      1_073_741_824,
      DELIVERY_DEFAULTS.maxQueueBytes,
    ),
    messagesPerSecond: bounded(
      "messagesPerSecond",
      delivery?.messagesPerSecond,
      0,
      10_000,
      DELIVERY_DEFAULTS.messagesPerSecond,
    ),
    maxAttempts: bounded(
      "maxAttempts",
      delivery?.maxAttempts,
      1,
      10,
      DELIVERY_DEFAULTS.maxAttempts,
    ),
    retryBaseMs,
    retryMaxMs,
    closeMs,
    totalMs,
  };
}

/**
 * Session reuse settings, passed to the native layer rather than the engine:
 * they govern how long one authenticated SMTP session may serve messages
 * (ADR-0022).
 */
function reuseSettings(delivery: DeliveryOptions | undefined): {
  maxMessagesPerSession: number;
  sessionIdleMs: number;
} {
  return {
    maxMessagesPerSession: bounded(
      "maxMessagesPerSession",
      delivery?.maxMessagesPerSession,
      1,
      10_000,
      DELIVERY_DEFAULTS.maxMessagesPerSession,
    ),
    sessionIdleMs: bounded(
      "sessionIdleMs",
      delivery?.sessionIdleMs,
      0,
      600_000,
      DELIVERY_DEFAULTS.sessionIdleMs,
    ),
  };
}

function addressString(value: Address): string {
  if (typeof value === "string") {
    return value;
  }
  return value.name ? `${value.name} <${value.address}>` : value.address;
}

function addressList(values: Address[] | undefined): string[] | undefined {
  return values?.map(addressString);
}

function headerPairs(headers: Message["headers"]): Array<{ name: string; value: string }> {
  if (!headers) {
    return [];
  }
  if (Array.isArray(headers)) {
    return headers.map((header) => ({ name: header.name, value: header.value }));
  }
  return Object.entries(headers).map(([name, value]) => ({ name, value }));
}

function toNativeOptions(options: TransportOptions): Record<string, unknown> {
  assertKnownKeys(options, TRANSPORT_KEYS);
  if (options.auth) assertKnownKeys(options.auth, AUTH_KEYS);
  if (options.timeouts) assertKnownKeys(options.timeouts, TIMEOUT_KEYS);
  if (options.limits) assertKnownKeys(options.limits, LIMIT_KEYS);
  if (options.attachments) assertKnownKeys(options.attachments, ATTACHMENT_KEYS);

  const native: Record<string, unknown> = {
    host: options.host,
    port: options.port,
    secure: options.secure,
  };

  if (options.auth) {
    native.auth = nativeAuth(options.auth);
  }
  if (options.ehloName !== undefined) {
    native.ehloName = options.ehloName;
  }
  if (options.tlsRootCertificate !== undefined) {
    native.tlsRootCertificate = options.tlsRootCertificate;
  }

  const timeouts = options.timeouts;
  if (timeouts) {
    if (timeouts.connectMs !== undefined) native.connectMs = timeouts.connectMs;
    if (timeouts.operationMs !== undefined) native.operationMs = timeouts.operationMs;
    if (timeouts.totalMs !== undefined) native.totalMs = timeouts.totalMs;
    if (timeouts.closeMs !== undefined) native.closeMs = timeouts.closeMs;
  }

  const limits = options.limits;
  if (limits) {
    if (limits.maxRecipients !== undefined) native.maxRecipients = limits.maxRecipients;
    if (limits.maxAttachmentBytes !== undefined) {
      native.maxAttachmentBytes = limits.maxAttachmentBytes;
    }
    if (limits.maxTotalAttachmentBytes !== undefined) {
      native.maxTotalAttachmentBytes = limits.maxTotalAttachmentBytes;
    }
    if (limits.maxWireBytes !== undefined) native.maxWireBytes = limits.maxWireBytes;
  }

  if (options.attachments?.allowFileAccess !== undefined) {
    native.allowFileAccess = options.attachments.allowFileAccess;
  }

  return native;
}

function toNativeAttachment(attachment: Attachment): Record<string, unknown> {
  if ("content" in attachment) {
    const native: Record<string, unknown> = {
      filename: attachment.filename,
      // Copied synchronously, before the native call can return: the caller may
      // reuse or mutate the source array as soon as send() has been called.
      content: new Uint8Array(attachment.content),
    };
    if (attachment.contentType !== undefined) {
      native.contentType = attachment.contentType;
    }
    return native;
  }

  const native: Record<string, unknown> = { path: attachment.path };
  if (attachment.filename !== undefined) native.filename = attachment.filename;
  if (attachment.contentType !== undefined) native.contentType = attachment.contentType;
  return native;
}

function toNativeMessage(
  message: Message,
  perFileBytes: number,
  totalBytes: number,
): Record<string, unknown> {
  // Sizes are checked before any buffer is copied. `send()` copies every
  // buffer synchronously, so an oversize payload must be rejected here rather
  // than after the whole of it has been duplicated in the JS heap.
  assertAttachmentBudget(message.attachments, perFileBytes, totalBytes);

  const native: Record<string, unknown> = {
    headers: headerPairs(message.headers),
  };

  // Omit absent required fields so the core reports its stable validation code
  // instead of the adapter failing on a conversion.
  if (message.from !== undefined) native.from = addressString(message.from);
  if (message.subject !== undefined) native.subject = message.subject;

  const to = addressList(message.to);
  if (to) native.to = to;
  const cc = addressList(message.cc);
  if (cc) native.cc = cc;
  const bcc = addressList(message.bcc);
  if (bcc) native.bcc = bcc;
  const replyTo = addressList(message.replyTo);
  if (replyTo) native.replyTo = replyTo;
  if (message.text !== undefined) native.text = message.text;
  if (message.html !== undefined) native.html = message.html;
  if (message.attachments) native.attachments = message.attachments.map(toNativeAttachment);

  return native;
}

/**
 * Rough byte budget for the queue: the text and HTML a caller supplied plus any
 * buffer attachment. It deliberately ignores encoding overhead, so it
 * under-counts rather than rejecting a message the core would accept.
 */
function estimateMessageBytes(message: Message): number {
  let bytes = 0;
  if (message.subject) bytes += message.subject.length;
  if (message.text) bytes += message.text.length;
  if (message.html) bytes += message.html.length;
  for (const attachment of message.attachments ?? []) {
    if ("content" in attachment) bytes += attachment.content.byteLength;
  }
  return bytes;
}

function rethrow(error: unknown): never {
  const mapped = fromNative(error);
  throw mapped ?? error;
}

/**
 * One open transport.
 *
 * Lifecycle is `open -> closing -> closed`. A send is queued and scheduled: the
 * engine runs up to `maxConcurrent` sends at once over at most `maxConnections`
 * native sessions, retries only transient failures, and never retries an
 * `unknown` disposition. `close()` is explicit, idempotent, bounded by `closeMs`
 * (125 s by default), and never driven by garbage collection or process exit.
 *
 * Every result means relay acceptance only, never delivery.
 */
export class Transport {
  readonly #engine: DeliveryEngine;
  /** Effective per-attachment cap: the caller's lowered value or the default. */
  readonly #perFileBytes: number;
  /** Effective aggregate attachment cap: the caller's lowered value or the default. */
  readonly #totalBytes: number;

  constructor(options: TransportOptions) {
    // Validate the caller's options before loading the addon, so an invalid
    // option always reports a Sendust error even where the addon is missing.
    const native = toNativeOptions(options);
    const reuse = reuseSettings(options.delivery);
    // Every option that can throw is validated before a native transport
    // exists, so a rejected value cannot leave one behind unclosed.
    const settings = deliverySettings(
      options.delivery,
      options.timeouts?.closeMs ?? CLOSE_MS_DEFAULT,
      options.timeouts?.totalMs ?? TOTAL_MS_DEFAULT,
    );
    const nativeOptions: Record<string, unknown> = {
      ...native,
      maxMessagesPerSession: reuse.maxMessagesPerSession,
      sessionIdleMs: reuse.sessionIdleMs,
    };
    // Snapshot the caps now; the caller's options object may be mutated later.
    this.#perFileBytes = options.limits?.maxAttachmentBytes ?? DEFAULT_LIMITS.maxAttachmentBytes;
    this.#totalBytes =
      options.limits?.maxTotalAttachmentBytes ?? DEFAULT_LIMITS.maxTotalAttachmentBytes;

    // One native transport is created eagerly: the core validates the relay at
    // construction, and a missing addon fails here rather than on first send.
    let first: NativeTransport;
    try {
      first = new (loadNative().Transport)(withFreshToken(nativeOptions, options));
    } catch (error) {
      rethrow(error);
    }

    this.#engine = new DeliveryEngine({
      ...settings,
      sessionIdleMs: reuse.sessionIdleMs,
      createTransport: () => {
        try {
          return new (loadNative().Transport)(withFreshToken(nativeOptions, options));
        } catch (error) {
          return rethrow(error);
        }
      },
      first,
    });
  }

  /** Current lifecycle state, read synchronously. */
  get state(): TransportState {
    return this.#engine.state;
  }

  /** Queue depth, active sends, pool size, and outcome counters. */
  get stats(): TransportStats {
    return this.#engine.stats;
  }

  /**
   * Queues one message and resolves with the relay's outcome for that message.
   *
   * Resolves for `accepted`, `rejected`, and `unknown`; `accepted` means the
   * relay took responsibility, not that a mailbox received the message. A
   * transient failure (`4xx`, connect or operation timeout, reset before DATA)
   * is retried within the attempt budget; `unknown` and permanent failures are
   * returned as they are. Never sends per-recipient messages.
   *
   * @throws {SendustError} for validation, configuration, limit (including
   * `LIMIT_QUEUE_FULL`), TLS, auth, timeout, closed, and internal failures.
   */
  async send(message: Message): Promise<SendResult> {
    // Refused synchronously once close has been requested, so the race between
    // `close()` and a send cannot depend on when the native call is polled.
    if (this.#engine.state !== "open") {
      throw closedError();
    }
    // Built before submitting: every buffer is copied now, so the caller may
    // mutate its message as soon as send() has been called.
    const nativeMessage = toNativeMessage(message, this.#perFileBytes, this.#totalBytes);
    return this.#engine.submit(nativeMessage, estimateMessageBytes(message));
  }

  /**
   * Closes the transport.
   *
   * Idempotent. Queued messages reject with `TRANSPORT_CLOSED`, an active send
   * is cancelled within the close budget (reporting `unknown` when the body may
   * already have reached the relay), and every promise settles. After it
   * resolves, `send()` rejects with `TRANSPORT_CLOSED`.
   */
  async close(): Promise<void> {
    await this.#engine.close();
  }
}

/**
 * Creates one transport. Relay options are validated synchronously.
 *
 * @throws {SendustError} with `CONFIG_*` or `VALIDATION_*` codes for invalid
 * options; TLS verification cannot be disabled and there is no plaintext mode.
 */
export function createTransport(options: TransportOptions): Transport {
  return new Transport(options);
}

export { SendustError };
