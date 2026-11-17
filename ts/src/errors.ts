/**
 * Public error classes and stable error categories.
 *
 * The native layer reports a redacted JSON payload; the message text is never a
 * contract. Branch on `code`, on `category`, or on the class — never by parsing
 * a message string. Every class extends `SendustError`, so a single catch can
 * handle all of them, while a narrower `catch` can select one category.
 */

export type ErrorCategory =
  | "validation"
  | "configuration"
  | "limit"
  | "tls"
  | "authentication"
  | "protocol"
  | "timeout"
  | "closed"
  | "busy"
  | "internal";

export interface ErrorPayload {
  readonly code: string;
  readonly category: ErrorCategory;
  readonly message: string;
  readonly details?: Record<string, unknown>;
}

const PAYLOAD_PREFIX = "SENDUST:";

/**
 * Catchable error raised by Sendust.
 *
 * Carries the stable `code`, its `category`, and the whitelisted `details`
 * (host, port, stage, recipient, SMTP codes). Credentials, body content,
 * attachment bytes, and absolute paths are never present.
 */
export class SendustError extends Error {
  readonly code: string;
  readonly category: ErrorCategory;
  readonly details: Record<string, unknown>;

  constructor(payload: ErrorPayload) {
    super(payload.message);
    this.name = new.target.name;
    this.code = payload.code;
    this.category = payload.category;
    this.details = payload.details ?? {};
  }
}

/** Malformed input: options, addresses, headers, message, attachments. */
export class SendustValidationError extends SendustError {}

/** Transport or attachment configuration that cannot be used as given. */
export class SendustConfigurationError extends SendustError {}

/** A hard cap was exceeded. */
export class SendustLimitError extends SendustError {}

/** Certificate, handshake, or protocol-version failure. */
export class SendustTlsError extends SendustError {}

/** The relay rejected the credentials or offered no supported mechanism. */
export class SendustAuthenticationError extends SendustError {}

/** The relay's behaviour broke the protocol. */
export class SendustProtocolError extends SendustError {}

/** A connect, operation, total, or close budget expired. */
export class SendustTimeoutError extends SendustError {}

/** The transport is closed or closing. */
export class SendustClosedError extends SendustError {}

/** Another send is already active on this transport. */
export class SendustBusyError extends SendustError {}

/** An invariant was violated; report a bug rather than retrying. */
export class SendustInternalError extends SendustError {}

const CATEGORY_CLASSES: Record<ErrorCategory, typeof SendustError> = {
  validation: SendustValidationError,
  configuration: SendustConfigurationError,
  limit: SendustLimitError,
  tls: SendustTlsError,
  authentication: SendustAuthenticationError,
  protocol: SendustProtocolError,
  timeout: SendustTimeoutError,
  closed: SendustClosedError,
  busy: SendustBusyError,
  internal: SendustInternalError,
};

function isErrorCategory(value: unknown): value is ErrorCategory {
  return typeof value === "string" && Object.hasOwn(CATEGORY_CLASSES, value);
}

/** Maps a native rejection to a typed `SendustError`, or null when foreign. */
export function fromNative(error: unknown): SendustError | null {
  if (!(error instanceof Error)) {
    return null;
  }

  // A facade-tagged error (for example the loader's CONFIG_NATIVE_UNAVAILABLE)
  // carries its stable code and category directly instead of a native payload.
  const tagged = error as { code?: unknown; sendustCategory?: unknown };
  if (
    typeof tagged.code === "string" &&
    isErrorCategory(tagged.sendustCategory) &&
    error.constructor === Error
  ) {
    const ErrorClass = CATEGORY_CLASSES[tagged.sendustCategory];
    return new ErrorClass({
      code: tagged.code,
      category: tagged.sendustCategory,
      message: error.message,
    });
  }

  const index = error.message.indexOf(PAYLOAD_PREFIX);
  if (index === -1) {
    return null;
  }
  try {
    const payload = JSON.parse(error.message.slice(index + PAYLOAD_PREFIX.length)) as ErrorPayload;
    if (!isErrorCategory(payload.category)) {
      return null;
    }
    const ErrorClass = CATEGORY_CLASSES[payload.category];
    return new ErrorClass(payload);
  } catch {
    return null;
  }
}
