/**
 * SMTP outcome classification and retry policy.
 *
 * Pure functions over native outcomes and typed errors, so the scheduler in
 * `engine.ts` never re-reads an SMTP reply and every retry decision is testable
 * without a relay.
 *
 * The rules encode the delivery contract: `accepted` and `unknown` are never
 * retried, a `4xx` final reply is a non-acceptance and may be retried, a `5xx`
 * is permanent, and rejected credentials stop the transport instead of being
 * hammered.
 */

import { SendustError } from "./errors.ts";
import type { SendResult } from "./types.ts";

/** Exponential backoff bounds, in milliseconds. */
export interface BackoffPolicy {
  readonly baseMs: number;
  readonly maxMs: number;
}

/** How many attempts a message has used and may use. */
export interface AttemptLedger {
  readonly attempt: number;
  readonly maxAttempts: number;
}

/** What a final SMTP reply means for retrying without duplicating a message. */
export type ReplyClass = "success" | "transient" | "permanent" | "unknown";

export type RetryDecision =
  | { readonly retry: false; readonly reason: string; readonly fatal?: boolean }
  | { readonly retry: true; readonly reason: string; readonly delayMs: number };

/**
 * Classifies a final reply.
 *
 * `4xx` is transient and means the relay did not accept the message, so a retry
 * cannot duplicate it. `5xx` is permanent. Anything else is not a final reply.
 */
export function classifyReply(code: number): ReplyClass {
  if (code >= 200 && code < 300) return "success";
  if (code >= 400 && code < 500) return "transient";
  if (code >= 500 && code < 600) return "permanent";
  return "unknown";
}

/**
 * Errors that describe a condition before the message could have been accepted.
 *
 * A failure after the DATA terminator is reported as the `unknown` outcome by
 * the core, never as one of these errors, so retrying them cannot duplicate a
 * message.
 */
const RETRYABLE_ERRORS: ReadonlySet<string> = new Set([
  "TIMEOUT_CONNECT",
  "TIMEOUT_OPERATION",
  "TIMEOUT_TOTAL",
  "PROTOCOL_CONNECTION_RESET",
]);

/** Rejected credentials are not transient: every further send fails the same way. */
const FATAL_ERRORS: ReadonlySet<string> = new Set(["AUTH_FAILED", "AUTH_UNSUPPORTED"]);

/**
 * Full-jitter backoff: half the window is fixed, half is random, so a burst of
 * retries spreads out instead of resynchronizing into a storm.
 */
export function backoffDelay(
  attempt: number,
  policy: BackoffPolicy,
  random: () => number = Math.random,
): number {
  const step = Math.max(0, attempt - 1);
  const cap = Math.min(policy.maxMs, policy.baseMs * 2 ** step);
  const half = cap / 2;
  return Math.max(0, Math.round(half + random() * half));
}

function exhausted(ledger: AttemptLedger): RetryDecision | null {
  return ledger.attempt >= ledger.maxAttempts
    ? { retry: false, reason: "attempt budget exhausted" }
    : null;
}

/** What to do after a send settled with a relay outcome. */
export function retryAfterResult(
  result: SendResult,
  ledger: AttemptLedger,
  policy: BackoffPolicy,
  random: () => number = Math.random,
): RetryDecision {
  if (result.status === "accepted") {
    return { retry: false, reason: "accepted" };
  }
  if (result.status === "unknown") {
    // The body may have reached the relay. A retry could duplicate it, so an
    // uncertain disposition is final by contract.
    return { retry: false, reason: "unknown disposition is never retried" };
  }

  const code = result.response?.code;
  if (code === undefined) {
    return { retry: false, reason: "rejection without a final reply" };
  }
  if (classifyReply(code) !== "transient") {
    return { retry: false, reason: `permanent reply ${code}` };
  }

  return (
    exhausted(ledger) ?? {
      retry: true,
      reason: `transient reply ${code}`,
      delayMs: backoffDelay(ledger.attempt, policy, random),
    }
  );
}

/** What to do after a send failed with an error instead of an outcome. */
export function retryAfterError(
  error: unknown,
  ledger: AttemptLedger,
  policy: BackoffPolicy,
  random: () => number = Math.random,
): RetryDecision {
  const code = error instanceof SendustError ? error.code : undefined;
  if (code === undefined) {
    return { retry: false, reason: "unclassified failure" };
  }
  if (FATAL_ERRORS.has(code)) {
    return { retry: false, reason: `fatal ${code}`, fatal: true };
  }
  if (!RETRYABLE_ERRORS.has(code)) {
    return { retry: false, reason: `not retryable: ${code}` };
  }

  return (
    exhausted(ledger) ?? {
      retry: true,
      reason: `transient error ${code}`,
      delayMs: backoffDelay(ledger.attempt, policy, random),
    }
  );
}

/** Defaults for the retry policy. */
export const DEFAULT_BACKOFF: BackoffPolicy = { baseMs: 250, maxMs: 30_000 };
