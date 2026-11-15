import { describe, expect, it } from "vitest";
import { toSendResult } from "../src/results.ts";

describe("result mapping", () => {
  it("maps accepted without adding a delivery claim", () => {
    const result = toSendResult({
      status: "accepted",
      recipients: [{ address: "a@example.test", status: "accepted" }],
      response: { code: 250, message: "2.0.0 OK" },
    });

    expect(result).toEqual({
      status: "accepted",
      recipients: [{ address: "a@example.test", status: "accepted" }],
      response: { code: 250, message: "2.0.0 OK" },
    });
    expect(Object.keys(result).sort()).toEqual(["recipients", "response", "status"]);
  });

  it("keeps rejected and unknown verbatim", () => {
    const rejected = toSendResult({
      status: "rejected",
      recipients: [{ address: "a@example.test", status: "rejected" }],
    });
    expect(rejected.status).toBe("rejected");
    expect(rejected.response).toBeUndefined();

    const unknown = toSendResult({
      status: "unknown",
      recipients: [{ address: "a@example.test", status: "unknown" }],
    });
    expect(unknown.status).toBe("unknown");
  });

  it("keeps mixed recipient statuses in order", () => {
    const result = toSendResult({
      status: "accepted",
      recipients: [
        { address: "a@example.test", status: "accepted" },
        { address: "b@example.test", status: "rejected" },
        { address: "c@example.test", status: "unknown" },
      ],
    });

    expect(result.recipients.map((recipient) => recipient.status)).toEqual([
      "accepted",
      "rejected",
      "unknown",
    ]);
  });

  it("rejects an undocumented native status", () => {
    expect(() =>
      toSendResult({
        status: "delivered",
        recipients: [{ address: "a@example.test", status: "accepted" }],
      }),
    ).toThrow(/unexpected native delivery status/);

    expect(() =>
      toSendResult({
        status: "accepted",
        recipients: [{ address: "a@example.test", status: "bounced" }],
      }),
    ).toThrow(/unexpected native recipient status/);
  });
});
