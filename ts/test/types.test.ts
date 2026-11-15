import { describe, expectTypeOf, it } from "vitest";
import {
  SendustError,
  Transport,
  createTransport,
  version,
  type Attachment,
  type DeliveryStatus,
  type Message,
  type RecipientOutcome,
  type SendResult,
  type TransportOptions,
  type TransportState,
} from "../src/index.ts";

/**
 * Compile-time contract tests. `tsc` type-checks this file (see
 * `ts/tsconfig.json`); `@ts-expect-error` lines fail the build if the
 * prohibited form ever becomes legal.
 */
describe("public contract types", () => {
  it("types the documented result shape", () => {
    expectTypeOf<SendResult["status"]>().toEqualTypeOf<DeliveryStatus>();
    expectTypeOf<SendResult["recipients"]>().toEqualTypeOf<RecipientOutcome[]>();
    expectTypeOf<RecipientOutcome["status"]>().toEqualTypeOf<DeliveryStatus>();
    expectTypeOf<DeliveryStatus>().toEqualTypeOf<"accepted" | "rejected" | "unknown">();
  });

  it("types the transport surface", () => {
    expectTypeOf(createTransport).parameter(0).toEqualTypeOf<TransportOptions>();
    expectTypeOf(createTransport).returns.toEqualTypeOf<Transport>();
    expectTypeOf<Transport["state"]>().toEqualTypeOf<TransportState>();
    expectTypeOf<TransportState>().toEqualTypeOf<"open" | "closing" | "closed">();
    expectTypeOf(version).toEqualTypeOf<string>();
  });

  it("keeps errors catchable with stable, typed fields", () => {
    const error = new SendustError({ code: "INTERNAL", category: "internal", message: "m" });
    expectTypeOf(error.code).toEqualTypeOf<string>();
    expectTypeOf(error.category).toEqualTypeOf<
      | "validation"
      | "configuration"
      | "limit"
      | "tls"
      | "authentication"
      | "protocol"
      | "timeout"
      | "closed"
      | "busy"
      | "internal"
    >();
    expectTypeOf(error).toMatchTypeOf<Error>();
  });

  it("rejects options outside the frozen surface", () => {
    // @ts-expect-error pooling is not part of the MVP surface
    createTransport({ host: "relay.test", port: 465, secure: "tls", pool: 4 });
    // @ts-expect-error there is no plaintext security mode
    createTransport({ host: "relay.test", port: 25, secure: "none" });
    createTransport({
      host: "relay.test",
      port: 465,
      secure: "tls",
      auth: {
        user: "u",
        pass: "p",
        // @ts-expect-error only PLAIN and LOGIN can be forced
        method: "cram-md5",
      },
    });
  });

  it("limits attachments to buffer and path shapes", () => {
    const buffer: Attachment = { filename: "a.txt", content: new Uint8Array([1]) };
    const file: Attachment = { path: "/tmp/a.txt" };
    expectTypeOf(buffer).toMatchTypeOf<Attachment>();
    expectTypeOf(file).toMatchTypeOf<Attachment>();

    // @ts-expect-error no URL or remote attachment form exists
    const remote: Attachment = { url: "https://example.test/a.txt" };
    void remote;
  });

  it("keeps state read-only and the subject required", () => {
    const mutate = (transport: Transport) => {
      // @ts-expect-error state is read-only
      transport.state = "closed";
    };
    void mutate;

    // @ts-expect-error subject is required
    const message: Message = { from: "a@example.test", text: "body" };
    void message;
  });
});
