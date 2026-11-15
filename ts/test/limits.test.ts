import { describe, expect, it } from "vitest";
import { DEFAULT_LIMITS, assertAttachmentBudget } from "../src/limits.ts";
import type { SendustError } from "../src/errors.ts";
import type { Attachment } from "../src/types.ts";

function buffer(size: number, filename = "file.bin"): Attachment {
  return { filename, content: new Uint8Array(size) };
}

function codeOf(run: () => void): string | null {
  try {
    run();
    return null;
  } catch (error) {
    return (error as SendustError).code;
  }
}

describe("attachment budget", () => {
  it("accepts sizes exactly at the caps", () => {
    const single = codeOf(() => assertAttachmentBudget([buffer(4)], 4, 8));
    const pair = codeOf(() => assertAttachmentBudget([buffer(4), buffer(4)], 4, 8));
    expect(single).toBeNull();
    expect(pair).toBeNull();
  });

  it("rejects a buffer one byte over the per-file cap", () => {
    const code = codeOf(() => assertAttachmentBudget([buffer(5)], 4, 100));
    expect(code).toBe("LIMIT_ATTACHMENT_BYTES");
  });

  it("rejects the aggregate when every file on its own fits", () => {
    const code = codeOf(() => assertAttachmentBudget([buffer(3), buffer(3)], 4, 5));
    expect(code).toBe("LIMIT_ATTACHMENT_TOTAL");
  });

  it("reports a per-file violation before an aggregate one", () => {
    const code = codeOf(() => assertAttachmentBudget([buffer(5), buffer(5)], 4, 6));
    expect(code).toBe("LIMIT_ATTACHMENT_BYTES");
  });

  it("ignores path attachments, whose size is unknown until read", () => {
    const code = codeOf(() => assertAttachmentBudget([{ path: "/tmp/a.bin" }], 1, 1));
    expect(code).toBeNull();
  });

  it("accepts an absent or empty list", () => {
    const absent = codeOf(() => assertAttachmentBudget(undefined, 1, 1));
    const empty = codeOf(() => assertAttachmentBudget([], 1, 1));
    expect(absent).toBeNull();
    expect(empty).toBeNull();
  });

  it("publishes the approved defaults in bytes", () => {
    expect(DEFAULT_LIMITS.maxRecipients).toBe(100);
    expect(DEFAULT_LIMITS.maxAttachmentBytes).toBe(10 * 1024 * 1024);
    expect(DEFAULT_LIMITS.maxTotalAttachmentBytes).toBe(20 * 1024 * 1024);
    expect(DEFAULT_LIMITS.maxWireBytes).toBe(30 * 1024 * 1024);
  });
});
