/**
 * Runtime-neutral transport facade.
 *
 * Owns lifecycle composition over the native binding only. SMTP, MIME,
 * validation, and delivery policy stay in `sendust-core`.
 */

import { SendustError, SendustValidationError, fromNative } from "./errors.ts";
import { DEFAULT_LIMITS, assertAttachmentBudget } from "./limits.ts";
import { loadNative, type NativeTransport } from "./loader.ts";
import { toSendResult } from "./results.ts";
import type {
  Address,
  Attachment,
  Message,
  SendResult,
  TransportOptions,
  TransportState,
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
]);
const AUTH_KEYS = new Set(["user", "pass", "method"]);
const TIMEOUT_KEYS = new Set(["connectMs", "operationMs", "totalMs", "closeMs"]);
const LIMIT_KEYS = new Set([
  "maxRecipients",
  "maxAttachmentBytes",
  "maxTotalAttachmentBytes",
  "maxWireBytes",
]);
const ATTACHMENT_KEYS = new Set(["allowFileAccess"]);

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
    native.auth = {
      user: options.auth.user,
      pass: options.auth.pass,
      method: options.auth.method,
    };
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

function rethrow(error: unknown): never {
  const mapped = fromNative(error);
  throw mapped ?? error;
}

/**
 * One open transport.
 *
 * Lifecycle is `open -> closing -> closed`. One send may be active at a time;
 * a second overlapping `send()` rejects with `TRANSPORT_BUSY` immediately and
 * never opens another connection or queues work. `close()` is explicit,
 * idempotent, and bounded by `closeMs` (125 s by default); it is never driven by
 * garbage collection or process exit.
 *
 * Every result means relay acceptance only, never delivery.
 */
export class Transport {
  readonly #native: NativeTransport;
  /** Effective per-attachment cap: the caller's lowered value or the default. */
  readonly #perFileBytes: number;
  /** Effective aggregate attachment cap: the caller's lowered value or the default. */
  readonly #totalBytes: number;

  constructor(options: TransportOptions) {
    // Validate the caller's options before loading the addon, so an invalid
    // option always reports a Sendust error even where the addon is missing.
    const native = toNativeOptions(options);
    // Snapshot the caps now; the caller's options object may be mutated later.
    this.#perFileBytes = options.limits?.maxAttachmentBytes ?? DEFAULT_LIMITS.maxAttachmentBytes;
    this.#totalBytes =
      options.limits?.maxTotalAttachmentBytes ?? DEFAULT_LIMITS.maxTotalAttachmentBytes;
    try {
      this.#native = new (loadNative().Transport)(native);
    } catch (error) {
      rethrow(error);
    }
  }

  /** Current lifecycle state, read synchronously. */
  get state(): TransportState {
    return this.#native.state() as TransportState;
  }

  /**
   * Sends one message and resolves with the relay's outcome.
   *
   * Resolves for `accepted`, `rejected`, and `unknown`; `accepted` means the
   * relay took responsibility, not that a mailbox received the message. Never
   * retries, never queues, and never sends per-recipient messages.
   *
   * @throws {SendustError} for validation, configuration, limit, TLS, auth,
   * timeout, closed, busy, and internal failures.
   */
  async send(message: Message): Promise<SendResult> {
    try {
      return toSendResult(
        await this.#native.send(toNativeMessage(message, this.#perFileBytes, this.#totalBytes)),
      );
    } catch (error) {
      rethrow(error);
    }
  }

  /**
   * Closes the transport, cancelling an in-flight send within the close budget.
   *
   * Idempotent; after it resolves, `send()` rejects with `TRANSPORT_CLOSED`.
   * Reports `TIMEOUT_CLOSE` if the drain budget is exceeded.
   */
  async close(): Promise<void> {
    try {
      await this.#native.close();
    } catch (error) {
      rethrow(error);
    }
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
