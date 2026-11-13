import { createServer } from "node:net";
import { describe, expect, it } from "vitest";
import { SendustError, createTransport, version } from "../src/index.ts";

let available = true;
try {
  const { loadNative } = await import("../src/loader.ts");
  loadNative();
} catch {
  available = false;
}

describe("sendust facade", () => {
  it("exposes the package version", () => {
    expect(typeof version).toBe("string");
  });

  it.skipIf(!available)("rejects invalid relay options with a stable code", () => {
    expect.assertions(2);
    try {
      createTransport({ host: "", port: 465, secure: "tls" });
    } catch (error) {
      expect(error).toBeInstanceOf(SendustError);
      expect((error as SendustError).code).toBe("CONFIG_INVALID_RELAY");
    }
  });

  it.skipIf(!available)("closes idempotently and refuses later sends", async () => {
    const transport = createTransport({ host: "127.0.0.1", port: 1, secure: "tls" });
    expect(transport.state).toBe("open");

    await transport.close();
    await transport.close();
    expect(transport.state).toBe("closed");

    await expect(
      transport.send({ from: "a@example.test", to: ["b@example.test"], subject: "x", text: "y" }),
    ).rejects.toBeInstanceOf(SendustError);
  });

  it.skipIf(!available)("maps a refused connection to a protocol error", async () => {
    const server = createServer();
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
    const address = server.address();
    const port = typeof address === "object" && address ? address.port : 1;
    await new Promise<void>((resolve) => server.close(() => resolve()));

    const transport = createTransport({ host: "127.0.0.1", port, secure: "tls" });
    let code: string | null = null;
    try {
      await transport.send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
      });
    } catch (error) {
      code = (error as SendustError).code;
    }
    await transport.close();

    expect(code).toBe("PROTOCOL_CONNECTION_RESET");
  });
});
