/**
 * Sendust public TypeScript entry point.
 *
 * The runtime-neutral facade over the Node-API binding: transport creation,
 * one send per call, explicit close, and stable error codes.
 *
 * `SendResult.status` reports relay acceptance, never delivery: `accepted`
 * means the SMTP relay took responsibility for the message. One transport runs
 * one send at a time; overlapping sends reject with `TRANSPORT_BUSY`.
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
} from "./types.ts";

/** Package version; kept in step with package.json at release time. */
export const version = "0.1.0-rc.1";
