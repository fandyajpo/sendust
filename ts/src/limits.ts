/**
 * Approved caps, mirrored for the facade's pre-flight check.
 *
 * The numbers must equal `sendust_core::Limits`
 * (`crates/sendust-core/src/config/mod.rs`, ADR-0009). A caller may lower a
 * cap but never raise it; the binding enforces that when the transport is
 * created. `scripts/check-architecture.mjs` compares this table with the Rust
 * defaults, so the two cannot drift.
 */

import { SendustLimitError } from "./errors.ts";
import type { Attachment } from "./types.ts";

/** One mebibyte, the unit used by the approved caps. */
export const MIB = 1024 * 1024;

/** Documented defaults, which are also the hard maximums. */
export const DEFAULT_LIMITS = Object.freeze({
  maxRecipients: 100,
  maxAttachmentBytes: 10 * MIB,
  maxTotalAttachmentBytes: 20 * MIB,
  maxWireBytes: 30 * MIB,
} as const);

/**
 * Rejects oversize buffer attachments before any byte is copied.
 *
 * `send()` copies every buffer synchronously, so the size has to be checked
 * first: copying a caller's oversized buffer and rejecting it afterwards would
 * allocate the whole payload for nothing.
 *
 * File attachments are not checked here. Their size is unknown until they are
 * read, and the core enforces the same caps as it reads.
 */
export function assertAttachmentBudget(
  attachments: readonly Attachment[] | undefined,
  perFile: number,
  total: number,
): void {
  if (!attachments || attachments.length === 0) {
    return;
  }

  // Per-file first, then the aggregate: the same precedence the core uses, so
  // an input that breaks both reports the same code whichever side sees it.
  let sum = 0;
  for (const attachment of attachments) {
    if (!("content" in attachment)) {
      continue;
    }
    const size = attachment.content.byteLength;
    if (size > perFile) {
      throw new SendustLimitError({
        code: "LIMIT_ATTACHMENT_BYTES",
        category: "limit",
        message: "attachment exceeds the per-file cap",
      });
    }
    sum += size;
  }
  if (sum > total) {
    throw new SendustLimitError({
      code: "LIMIT_ATTACHMENT_TOTAL",
      category: "limit",
      message: "attachments exceed the aggregate cap",
    });
  }
}
