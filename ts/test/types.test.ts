import { describe, expectTypeOf, it } from "vitest";
import {
  SendustError,
  createTransport,
  version,
  type Attachment,
  type AuthOptions,
  type DeliveryStatus,
  type LimitOptions,
  type Message,
  type RecipientOutcome,
  type SendResult,
  type TimeoutOptions,
  type Transport,
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

  it("keeps the option and message surfaces exactly as documented", () => {
    expectTypeOf<keyof TransportOptions>().toEqualTypeOf<
      | "host"
      | "port"
      | "secure"
      | "auth"
      | "ehloName"
      | "timeouts"
      | "limits"
      | "attachments"
      | "tlsRootCertificate"
      | "delivery"
    >();
    expectTypeOf<keyof AuthOptions>().toEqualTypeOf<"user" | "pass" | "method">();
    expectTypeOf<keyof TimeoutOptions>().toEqualTypeOf<
      "connectMs" | "operationMs" | "totalMs" | "closeMs"
    >();
    expectTypeOf<keyof LimitOptions>().toEqualTypeOf<
      "maxRecipients" | "maxAttachmentBytes" | "maxTotalAttachmentBytes" | "maxWireBytes"
    >();
    expectTypeOf<keyof Message>().toEqualTypeOf<
      | "from"
      | "to"
      | "cc"
      | "bcc"
      | "replyTo"
      | "subject"
      | "text"
      | "html"
      | "headers"
      | "attachments"
    >();
  });

  it("rejects options outside the frozen surface", () => {
    // Never called: these lines exist so `tsc` checks the negative cases.
    const invalid = () => {
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
    };
    void invalid;
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
