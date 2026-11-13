/**
 * Public TypeScript types for transport options, messages, and results.
 *
 * These mirror the approved contract in docs/architecture/public-api.md. The
 * authoritative declarations are generated with the public API in P09.
 */

export type DeliveryStatus = "accepted" | "rejected" | "unknown";
export type RecipientStatus = DeliveryStatus;

export interface SmtpResponse {
  readonly code: number;
  readonly message: string;
}

export interface RecipientOutcome {
  readonly address: string;
  readonly status: RecipientStatus;
}

export interface SendResult {
  readonly status: DeliveryStatus;
  readonly recipients: RecipientOutcome[];
  readonly response?: SmtpResponse;
}

export interface AuthOptions {
  readonly user: string;
  readonly pass: string;
}

export interface TimeoutOptions {
  readonly connectMs?: number;
  readonly operationMs?: number;
  readonly totalMs?: number;
  readonly closeMs?: number;
}

export interface LimitOptions {
  readonly maxRecipients?: number;
  readonly maxAttachmentBytes?: number;
  readonly maxTotalAttachmentBytes?: number;
  readonly maxWireBytes?: number;
}

export interface TransportOptions {
  readonly host: string;
  readonly port: number;
  readonly secure: "tls" | "starttls";
  readonly auth?: AuthOptions;
  readonly ehloName?: string;
  readonly timeouts?: TimeoutOptions;
  readonly limits?: LimitOptions;
  readonly attachments?: { readonly allowFileAccess?: boolean };
}

export interface MailboxLike {
  readonly name?: string;
  readonly address: string;
}

export type Address = string | MailboxLike;

export interface HeaderLike {
  readonly name: string;
  readonly value: string;
}

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
}

export type TransportState = "open" | "closing" | "closed";
