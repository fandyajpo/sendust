/**
 * Sendust public TypeScript entry point.
 *
 * The runtime-neutral facade over the Node-API binding: transport creation,
 * automatic queueing and pooling, explicit close, and stable error codes.
 *
 * `SendResult.status` reports relay acceptance, never delivery: `accepted`
 * means the SMTP relay took responsibility for the message. Concurrent sends
 * are queued and scheduled automatically.
 */

export { createTransport, Transport } from "./transport.ts";
export { DEFAULT_LIMITS, MIB } from "./limits.ts";
export {
  SendustAuthenticationError,
  SendustBusyError,
  SendustClosedError,
  SendustConfigurationError,
  SendustError,
  SendustInternalError,
  SendustLimitError,
  SendustProtocolError,
  SendustTimeoutError,
  SendustTlsError,
  SendustValidationError,
} from "./errors.ts";
export type { ErrorCategory } from "./errors.ts";
export type {
  Address,
  Attachment,
  AuthOptions,
  BufferAttachment,
  DeliveryOptions,
  DeliveryStatus,
  FileAttachment,
  HeaderLike,
  LimitOptions,
  MailboxLike,
  Message,
  RecipientOutcome,
  RecipientStatus,
  SendResult,
  SmtpResponse,
  TimeoutOptions,
  TransportOptions,
  TransportState,
  TransportStats,
} from "./types.ts";

/** Package version; kept in step with package.json at release time. */
export const version = "1.0.1";
