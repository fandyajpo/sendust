/**
 * Runtime-neutral transport facade.
 *
 * Owns lifecycle composition over the native binding only. SMTP, MIME,
 * validation, and delivery policy stay in `sendust-core`.
 */

import { SendustError, fromNative } from "./errors.ts";
import { loadNative, type NativeSendResult, type NativeTransport } from "./loader.ts";
import type { Address, Message, SendResult, TransportOptions, TransportState } from "./types.ts";

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
  const native: Record<string, unknown> = {
    host: options.host,
    port: options.port,
    secure: options.secure,
  };

  if (options.auth) {
    native.auth = { user: options.auth.user, pass: options.auth.pass };
  }
  if (options.ehloName !== undefined) {
    native.ehloName = options.ehloName;
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

function toNativeMessage(message: Message): Record<string, unknown> {
  const native: Record<string, unknown> = {
    from: addressString(message.from),
    subject: message.subject,
    headers: headerPairs(message.headers),
  };

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

  return native;
}

function toResult(result: NativeSendResult): SendResult {
  const mapped: SendResult = {
    status: result.status as SendResult["status"],
    recipients: result.recipients.map((recipient) => ({
      address: recipient.address,
      status: recipient.status as SendResult["recipients"][number]["status"],
    })),
  };
  if (result.response) {
    return { ...mapped, response: result.response };
  }
  return mapped;
}

function rethrow(error: unknown): never {
  const mapped = fromNative(error);
  throw mapped ?? error;
}

/** One open transport; one active send; explicit bounded close. */
export class Transport {
  readonly #native: NativeTransport;

  constructor(options: TransportOptions) {
    try {
      this.#native = new (loadNative().Transport)(toNativeOptions(options));
    } catch (error) {
      rethrow(error);
    }
  }

  get state(): TransportState {
    return this.#native.state() as TransportState;
  }

  async send(message: Message): Promise<SendResult> {
    try {
      return toResult(await this.#native.send(toNativeMessage(message)));
    } catch (error) {
      rethrow(error);
    }
  }

  async close(): Promise<void> {
    try {
      await this.#native.close();
    } catch (error) {
      rethrow(error);
    }
  }
}

/** Creates one transport. Throws `SendustError` on invalid options. */
export function createTransport(options: TransportOptions): Transport {
  return new Transport(options);
}

export { SendustError };
