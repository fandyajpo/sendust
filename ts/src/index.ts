/**
 * Sendust public TypeScript entry point.
 *
 * The runtime-neutral facade over the Node-API binding: transport creation,
 * one send per call, explicit close, and stable error codes.
 */

export { createTransport, Transport } from "./transport.ts";
export { SendustError } from "./errors.ts";
export type { ErrorCategory } from "./errors.ts";
export type {
  Address,
  AuthOptions,
  DeliveryStatus,
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
export const version = "0.1.0";
