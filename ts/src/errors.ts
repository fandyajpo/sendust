/**
 * Public error class and stable error code re-exports.
 *
 * The native layer reports a redacted JSON payload; the message text is never a
 * contract. Branch on `code`/`category`, exactly as the error model requires.
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

export class SendustError extends Error {
  readonly code: string;
  readonly category: ErrorCategory;
  readonly details: Record<string, unknown>;

  constructor(payload: ErrorPayload) {
    super(payload.message);
    this.name = "SendustError";
    this.code = payload.code;
    this.category = payload.category;
    this.details = payload.details ?? {};
  }
}

/** Maps a native rejection to a `SendustError`, or returns null when foreign. */
export function fromNative(error: unknown): SendustError | null {
  if (!(error instanceof Error)) {
    return null;
  }
  const index = error.message.indexOf(PAYLOAD_PREFIX);
  if (index === -1) {
    return null;
  }
  try {
    const payload = JSON.parse(error.message.slice(index + PAYLOAD_PREFIX.length)) as ErrorPayload;
    return new SendustError(payload);
  } catch {
    return null;
  }
}
