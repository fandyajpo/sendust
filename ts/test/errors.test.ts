import { describe, expect, it } from "vitest";
import {
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
  createTransport,
  type ErrorCategory,
} from "../src/index.ts";
import type { ErrorPayload } from "../src/errors.ts";
import { fromNative } from "../src/errors.ts";

const CLASSES: Record<ErrorCategory, new (payload: ErrorPayload) => SendustError> = {
  validation: SendustValidationError,
  configuration: SendustConfigurationError,
  limit: SendustLimitError,
  tls: SendustTlsError,
  authentication: SendustAuthenticationError,
  protocol: SendustProtocolError,
  timeout: SendustTimeoutError,
  closed: SendustClosedError,
  busy: SendustBusyError,
  internal: SendustInternalError,
};

/** Every stable code and the category it must map to (docs/architecture/error-model.md). */
const CODES: Array<[string, ErrorCategory]> = [
  ["VALIDATION_INVALID_OPTION", "validation"],
  ["VALIDATION_INVALID_TIMEOUT", "validation"],
  ["VALIDATION_MISSING_FROM", "validation"],
  ["VALIDATION_MISSING_RECIPIENT", "validation"],
  ["VALIDATION_INVALID_ADDRESS", "validation"],
  ["VALIDATION_NON_ASCII_LOCAL_PART", "validation"],
  ["VALIDATION_INVALID_HEADER", "validation"],
  ["VALIDATION_HEADER_INJECTION", "validation"],
  ["VALIDATION_INVALID_ATTACHMENT", "validation"],
  ["CONFIG_UNSUPPORTED_SECURITY", "configuration"],
  ["CONFIG_UNSUPPORTED_AUTH", "configuration"],
  ["CONFIG_FILE_ACCESS_DISABLED", "configuration"],
  ["CONFIG_ATTACHMENT_UNREADABLE", "configuration"],
  ["CONFIG_INVALID_RELAY", "configuration"],
  ["CONFIG_NATIVE_UNAVAILABLE", "configuration"],
  ["LIMIT_RECIPIENTS", "limit"],
  ["LIMIT_ATTACHMENT_BYTES", "limit"],
  ["LIMIT_ATTACHMENT_TOTAL", "limit"],
  ["LIMIT_WIRE_BYTES", "limit"],
  ["TLS_CERTIFICATE", "tls"],
  ["TLS_HANDSHAKE", "tls"],
  ["TLS_PROTOCOL", "tls"],
  ["AUTH_FAILED", "authentication"],
  ["AUTH_UNSUPPORTED", "authentication"],
  ["PROTOCOL_REJECTED", "protocol"],
  ["PROTOCOL_UNEXPECTED_RESPONSE", "protocol"],
  ["PROTOCOL_CONNECTION_RESET", "protocol"],
  ["TIMEOUT_CONNECT", "timeout"],
  ["TIMEOUT_OPERATION", "timeout"],
  ["TIMEOUT_TOTAL", "timeout"],
  ["TIMEOUT_CLOSE", "timeout"],
  ["TRANSPORT_CLOSED", "closed"],
  ["TRANSPORT_BUSY", "busy"],
  ["INTERNAL", "internal"],
];

function nativeError(payload: ErrorPayload): Error {
  return new Error(`SENDUST:${JSON.stringify(payload)}`);
}

/** Everything a caller or a logger could see from one error. */
function serialize(error: unknown): string {
  if (!(error instanceof Error)) {
    return String(error);
  }
  return [error.name, error.message, error.stack ?? "", JSON.stringify(error)].join("\n");
}

let available = true;
try {
  const { loadNative } = await import("../src/loader.ts");
  loadNative();
} catch {
  available = false;
}

describe("public error classes", () => {
  it("covers every stable code with its category and class", () => {
    expect(CODES).toHaveLength(34);
    for (const [code, category] of CODES) {
      const error = fromNative(nativeError({ code, category, message: "diagnostic" }));
      expect(error).toBeInstanceOf(CLASSES[category]);
      expect(error).toBeInstanceOf(SendustError);
      expect(error).toBeInstanceOf(Error);
      expect(error?.code).toBe(code);
      expect(error?.category).toBe(category);
    }
  });

  it("maps a facade-tagged native-unavailable error to a configuration error", () => {
    const tagged = new Error("Sendust native addon is unavailable (sendust.node not found)");
    Object.defineProperty(tagged, "code", { value: "CONFIG_NATIVE_UNAVAILABLE" });
    Object.defineProperty(tagged, "sendustCategory", { value: "configuration" });

    const error = fromNative(tagged);
    expect(error).toBeInstanceOf(SendustConfigurationError);
    expect(error?.code).toBe("CONFIG_NATIVE_UNAVAILABLE");
    expect(error?.category).toBe("configuration");
  });

  it("leaves an ordinary error untouched", () => {
    const plain = new Error("something else");
    expect(fromNative(plain)).toBeNull();
  });

  it("keeps the class hierarchy narrow enough to catch one category", () => {
    const error = new SendustProtocolError({
      code: "PROTOCOL_CONNECTION_RESET",
      category: "protocol",
      message: "reset",
    });
    expect(error.name).toBe("SendustProtocolError");
    expect(error).toBeInstanceOf(SendustProtocolError);
    expect(error).not.toBeInstanceOf(SendustValidationError);
  });

  it.skipIf(!available)("redacts credentials, bodies, and paths from every error", async () => {
    const transport = createTransport({
      host: "127.0.0.1",
      port: 1,
      secure: "tls",
      auth: { user: "user", pass: "SECRET_PASS_MARKER" },
      delivery: { maxAttempts: 1 },
    });

    let sendError: unknown = null;
    try {
      await transport.send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "SECRET_BODY_MARKER",
      });
    } catch (error) {
      sendError = error;
    }
    await transport.close();

    let pathError: unknown = null;
    const pathTransport = createTransport({ host: "127.0.0.1", port: 1, secure: "tls" });
    try {
      await pathTransport.send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
        attachments: [{ path: "/tmp/SECRET_PATH_MARKER.txt" }],
      });
    } catch (error) {
      pathError = error;
    }
    await pathTransport.close();

    const rendered = `${serialize(sendError)}\n${serialize(pathError)}`;
    expect(sendError).toBeInstanceOf(SendustProtocolError);
    expect((sendError as SendustError).code).toBe("PROTOCOL_CONNECTION_RESET");
    expect(pathError).toBeInstanceOf(SendustConfigurationError);
    expect((pathError as SendustError).code).toBe("CONFIG_FILE_ACCESS_DISABLED");
    for (const marker of ["SECRET_PASS_MARKER", "SECRET_BODY_MARKER", "SECRET_PATH_MARKER"]) {
      expect(rendered).not.toContain(marker);
    }
  });

  it("returns null for a foreign error", () => {
    expect(fromNative(new Error("plain"))).toBeNull();
    expect(fromNative("not an error")).toBeNull();
    expect(
      fromNative(nativeError({ code: "INTERNAL", category: "internal", message: "m" })),
    ).toBeInstanceOf(SendustInternalError);
  });
});
