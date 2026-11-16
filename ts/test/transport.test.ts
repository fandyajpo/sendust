import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createServer, type Socket } from "node:net";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { createServer as createTlsServer } from "node:tls";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  SendustError,
  SendustValidationError,
  createTransport,
  version,
  type Attachment,
} from "../src/index.ts";

let available = true;
try {
  const { loadNative } = await import("../src/loader.ts");
  loadNative();
} catch {
  available = false;
}

/** Writes a fixture file in its own temp directory and returns its path. */
function tempFile(name: string, len: number): string {
  const dir = mkdtempSync(join(tmpdir(), "sendust-test-"));
  const path = join(dir, name);
  writeFileSync(path, new Uint8Array(len).fill(97));
  return path;
}

function removeTemp(path: string): void {
  rmSync(dirname(path), { recursive: true, force: true });
}

/**
 * Collects accepted sockets and resolves once the relay takes its first
 * connection, so a test can sequence on an event instead of sleeping.
 */
function connectionGate() {
  const sockets = new Set<Socket>();
  let open: (() => void) | undefined;
  const first = new Promise<void>((resolve) => {
    open = resolve;
  });
  const accept = (socket: Socket) => {
    sockets.add(socket);
    socket.on("close", () => sockets.delete(socket));
    open?.();
    open = undefined;
  };
  return { sockets, first, accept };
}

/** Resolves with the stable error code, or null when the send succeeded. */
async function errorCode(promise: Promise<unknown>): Promise<string | null> {
  try {
    await promise;
    return null;
  } catch (error) {
    return error instanceof SendustError ? error.code : null;
  }
}

/** Reads one committed TLS fixture relative to the repository root. */
function tlsFixture(name: string): string {
  const path = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "tests", "fixtures", "tls");
  return readFileSync(join(path, name), "utf8");
}

/**
 * A loopback SMTP relay over TLS using the committed valid fixture, so a test
 * with the test CA as an added root can observe an accepted send (ADR-0018).
 */
async function fakeTlsRelay(): Promise<{ port: number; close: () => Promise<void> }> {
  const sockets = new Set<Socket>();
  const server = createTlsServer(
    { key: tlsFixture("valid-server.key"), cert: tlsFixture("valid-server.crt") },
    (socket) => {
      sockets.add(socket);
      socket.on("close", () => sockets.delete(socket));
      socket.setEncoding("utf8");
      let buffer = "";
      let inData = false;
      socket.on("data", (chunk: string) => {
        buffer += chunk;
        for (;;) {
          const index = buffer.indexOf("\r\n");
          if (index === -1) {
            return;
          }
          const line = buffer.slice(0, index);
          buffer = buffer.slice(index + 2);
          if (inData) {
            if (line === ".") {
              inData = false;
              socket.write("250 2.0.0 accepted\r\n");
            }
            continue;
          }
          if (line.startsWith("EHLO")) {
            socket.write("250-sendust.test\r\n250 8BITMIME\r\n");
          } else if (line.startsWith("MAIL")) {
            socket.write("250 2.1.0 ok\r\n");
          } else if (line.startsWith("RCPT")) {
            socket.write("250 2.1.5 ok\r\n");
          } else if (line === "DATA") {
            inData = true;
            socket.write("354 go\r\n");
          } else if (line.startsWith("QUIT")) {
            socket.write("221 2.0.0 bye\r\n");
            socket.end();
          }
        }
      });
    },
  );
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  const port = typeof address === "object" && address ? address.port : 1;
  return {
    port,
    close: async () => {
      for (const socket of sockets) {
        socket.destroy();
      }
      await new Promise<void>((resolve) => server.close(() => resolve()));
    },
  };
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

  it.skipIf(!available)("closes concurrently without drifting past closed", async () => {
    const transport = createTransport({ host: "127.0.0.1", port: 1, secure: "tls" });

    await Promise.all([transport.close(), transport.close()]);

    expect(transport.state).toBe("closed");
  });

  it.skipIf(!available)("aborts a send in flight and rejects it as TRANSPORT_CLOSED", async () => {
    // Accept the TCP connection and stay silent: the client is left waiting in
    // the TLS handshake, which only close can interrupt.
    const gate = connectionGate();
    const server = createServer(gate.accept);
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
    const address = server.address();
    const port = typeof address === "object" && address ? address.port : 1;

    const transport = createTransport({
      host: "127.0.0.1",
      port,
      secure: "tls",
    });

    const sending = transport
      .send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
      })
      .catch((error: unknown) => error);

    // Let the delivery reach the handshake before close races it. The gate
    // resolves on the accepted connection, so no sleep is needed.
    await gate.first;
    await transport.close();

    const error = await sending;

    expect(error).toBeInstanceOf(SendustError);
    expect((error as SendustError).code).toBe("TRANSPORT_CLOSED");
    expect(transport.state).toBe("closed");

    for (const socket of gate.sockets) {
      socket.destroy();
    }
    await new Promise<void>((resolve) => server.close(() => resolve()));
  });

  it.skipIf(!available)("rejects a path attachment without filesystem opt-in", async () => {
    const transport = createTransport({ host: "127.0.0.1", port: 1, secure: "tls" });
    let code: string | null = null;
    try {
      await transport.send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
        attachments: [{ path: "/tmp/notes.txt", filename: "notes.txt" }],
      });
    } catch (error) {
      code = (error as SendustError).code;
    }
    await transport.close();

    expect(code).toBe("CONFIG_FILE_ACCESS_DISABLED");
  });

  it.skipIf(!available)("copies a buffer attachment before the send returns", async () => {
    const server = createServer();
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
    const address = server.address();
    const port = typeof address === "object" && address ? address.port : 1;
    await new Promise<void>((resolve) => server.close(() => resolve()));

    const transport = createTransport({ host: "127.0.0.1", port, secure: "tls" });
    const content = new Uint8Array([104, 101, 108, 108, 111]);
    let code: string | null = null;
    try {
      const sending = transport.send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
        attachments: [{ filename: "hello.txt", content }],
      });
      // The facade snapshots the array synchronously, so this must not matter.
      content.fill(0);
      await sending;
    } catch (error) {
      code = (error as SendustError).code;
    }
    await transport.close();

    expect(code).toBe("PROTOCOL_CONNECTION_RESET");
  });

  it.skipIf(!available)("rejects an attachment with neither a path nor content", async () => {
    const transport = createTransport({ host: "127.0.0.1", port: 1, secure: "tls" });
    let code: string | null = null;
    try {
      await transport.send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
        attachments: [{ filename: "empty.txt" } as unknown as Attachment],
      });
    } catch (error) {
      code = (error as SendustError).code;
    }
    await transport.close();

    expect(code).toBe("VALIDATION_INVALID_ATTACHMENT");
  });

  it.skipIf(!available)("sends a path attachment when filesystem access is enabled", async () => {
    const path = tempFile("path.txt", 3);
    const transport = createTransport({
      host: "127.0.0.1",
      port: 1,
      secure: "tls",
      attachments: { allowFileAccess: true },
    });

    let code: string | null = null;
    try {
      await transport.send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
        attachments: [{ path }],
      });
    } catch (error) {
      code = (error as SendustError).code;
    }
    await transport.close();
    removeTemp(path);

    expect(code).toBe("PROTOCOL_CONNECTION_RESET");
  });

  it.skipIf(!available)("enforces the per-file cap for a path attachment", async () => {
    const path = tempFile("big.bin", 3);
    const transport = createTransport({
      host: "127.0.0.1",
      port: 1,
      secure: "tls",
      attachments: { allowFileAccess: true },
      limits: { maxAttachmentBytes: 2 },
    });

    let code: string | null = null;
    try {
      await transport.send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
        attachments: [{ path }],
      });
    } catch (error) {
      code = (error as SendustError).code;
    }
    await transport.close();
    removeTemp(path);

    expect(code).toBe("LIMIT_ATTACHMENT_BYTES");
  });

  it.skipIf(!available)("enforces the aggregate cap across path attachments", async () => {
    const first = tempFile("a.bin", 3);
    const second = tempFile("b.bin", 3);
    const transport = createTransport({
      host: "127.0.0.1",
      port: 1,
      secure: "tls",
      attachments: { allowFileAccess: true },
      limits: { maxTotalAttachmentBytes: 4 },
    });

    let code: string | null = null;
    try {
      await transport.send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
        attachments: [{ path: first }, { path: second }],
      });
    } catch (error) {
      code = (error as SendustError).code;
    }
    await transport.close();
    removeTemp(first);
    removeTemp(second);

    expect(code).toBe("LIMIT_ATTACHMENT_TOTAL");
  });

  it.skipIf(!available)("snapshots a buffer before the caller detaches it", async () => {
    const transport = createTransport({ host: "127.0.0.1", port: 1, secure: "tls" });
    const content = new Uint8Array([1, 2, 3]);
    let code: string | null = null;
    try {
      const sending = transport.send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
        attachments: [{ filename: "detach.bin", content }],
      });
      // The facade copies synchronously; transferring the buffer away must not
      // change what the send already holds.
      structuredClone(content.buffer, { transfer: [content.buffer] });
      expect(content.byteLength).toBe(0);
      await sending;
    } catch (error) {
      code = (error as SendustError).code;
    }
    await transport.close();

    expect(code).toBe("PROTOCOL_CONNECTION_RESET");
  });

  it("rejects unknown option keys before touching the relay", () => {
    expect(() =>
      createTransport({
        host: "127.0.0.1",
        port: 1,
        secure: "tls",
        pool: 4,
      } as unknown as Parameters<typeof createTransport>[0]),
    ).toThrow(SendustValidationError);

    expect(() =>
      createTransport({
        host: "127.0.0.1",
        port: 1,
        secure: "tls",
        limits: { maxWireBytes: 10, bogus: 1 },
      } as unknown as Parameters<typeof createTransport>[0]),
    ).toThrow(SendustValidationError);
  });

  it.skipIf(!available)("rejects an unsupported auth method", async () => {
    const code = await errorCode(
      Promise.resolve().then(() =>
        createTransport({
          host: "127.0.0.1",
          port: 1,
          secure: "tls",
          auth: { user: "u", pass: "p", method: "cram-md5" },
        } as never),
      ),
    );
    expect(code).toBe("VALIDATION_INVALID_OPTION");
  });

  it.skipIf(!available)("validates a message before opening a connection", async () => {
    const transport = createTransport({ host: "127.0.0.1", port: 1, secure: "tls" });

    const withoutRecipient = await errorCode(
      transport.send({ from: "a@example.test", subject: "x", text: "y" }),
    );
    const withoutBody = await errorCode(
      transport.send({ from: "a@example.test", to: ["b@example.test"], subject: "x" }),
    );
    const withoutSubject = await errorCode(
      transport.send({ from: "a@example.test", to: ["b@example.test"], subject: "  ", text: "y" }),
    );
    await transport.close();

    expect(withoutRecipient).toBe("VALIDATION_MISSING_RECIPIENT");
    expect(withoutBody).toBe("VALIDATION_INVALID_OPTION");
    expect(withoutSubject).toBe("VALIDATION_INVALID_OPTION");
    expect(transport.state).toBe("closed");
  });

  it.skipIf(!available)("reports a missing From as VALIDATION_MISSING_FROM", async () => {
    const transport = createTransport({ host: "127.0.0.1", port: 1, secure: "tls" });
    const code = await errorCode(
      transport.send({ to: ["b@example.test"], subject: "x", text: "y" } as never),
    );
    await transport.close();

    expect(code).toBe("VALIDATION_MISSING_FROM");
  });

  it.skipIf(!available)("closes a stalled send within the close budget", async () => {
    const gate = connectionGate();
    const server = createServer(gate.accept);
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
    const address = server.address();
    const port = typeof address === "object" && address ? address.port : 1;

    const transport = createTransport({
      host: "127.0.0.1",
      port,
      secure: "tls",
      timeouts: { connectMs: 1000, operationMs: 1000, totalMs: 1000, closeMs: 1000 },
    });
    const sending = transport
      .send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
      })
      .catch((error: unknown) => error);
    await gate.first;

    const started = Date.now();
    await transport.close();
    const elapsed = Date.now() - started;

    expect(elapsed).toBeLessThan(1500);
    expect(transport.state).toBe("closed");
    expect(await sending).toBeInstanceOf(SendustError);

    for (const socket of gate.sockets) {
      socket.destroy();
    }
    await new Promise<void>((resolve) => server.close(() => resolve()));
  });

  it.skipIf(!available)("rejects a second concurrent send with TRANSPORT_BUSY", async () => {
    const gate = connectionGate();
    let connections = 0;
    const server = createServer((socket) => {
      connections += 1;
      gate.accept(socket);
    });
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
    const address = server.address();
    const port = typeof address === "object" && address ? address.port : 1;

    const transport = createTransport({ host: "127.0.0.1", port, secure: "tls" });
    const first = transport
      .send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
      })
      .catch((error: unknown) => error);
    await gate.first;

    const second = await errorCode(
      transport.send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
      }),
    );
    expect(second).toBe("TRANSPORT_BUSY");
    expect(connections).toBe(1);

    await transport.close();
    const firstError = await first;
    expect(firstError).toBeInstanceOf(SendustError);
    expect((firstError as SendustError).code).toBe("TRANSPORT_CLOSED");

    for (const socket of gate.sockets) {
      socket.destroy();
    }
    await new Promise<void>((resolve) => server.close(() => resolve()));
  });

  it.skipIf(!available)("refuses a send that starts during close", async () => {
    const gate = connectionGate();
    const server = createServer(gate.accept);
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
    const address = server.address();
    const port = typeof address === "object" && address ? address.port : 1;

    const transport = createTransport({ host: "127.0.0.1", port, secure: "tls" });
    const first = transport
      .send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
      })
      .catch((error: unknown) => error);
    await gate.first;

    const closing = transport.close();
    const during = await errorCode(
      transport.send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
      }),
    );
    await closing;

    expect(during).toBe("TRANSPORT_CLOSED");
    expect(transport.state).toBe("closed");
    expect(await first).toBeInstanceOf(SendustError);

    for (const socket of gate.sockets) {
      socket.destroy();
    }
    await new Promise<void>((resolve) => server.close(() => resolve()));
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

  it.skipIf(!available)("rejects an oversize attachment before copying it", async () => {
    const transport = createTransport({ host: "127.0.0.1", port: 1, secure: "tls" });
    // Only `byteLength` is real. Copying this would produce an empty buffer, so
    // a limit rejection proves the size was checked before the copy and not by
    // the native layer afterwards.
    const oversize = { byteLength: 10 * 1024 * 1024 + 1 } as unknown as Uint8Array;

    const code = await errorCode(
      transport.send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
        attachments: [{ filename: "big.bin", content: oversize }],
      }),
    );
    await transport.close();

    expect(code).toBe("LIMIT_ATTACHMENT_BYTES");
  });

  it.skipIf(!available)("honours a lowered attachment cap in the facade", async () => {
    const transport = createTransport({
      host: "127.0.0.1",
      port: 1,
      secure: "tls",
      limits: { maxAttachmentBytes: 4 },
    });

    const code = await errorCode(
      transport.send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
        attachments: [{ filename: "five.bin", content: new Uint8Array(5) }],
      }),
    );
    await transport.close();

    expect(code).toBe("LIMIT_ATTACHMENT_BYTES");
  });

  it.skipIf(!available)("trusts an added root and accepts a fake-relay send", async () => {
    const relay = await fakeTlsRelay();
    const transport = createTransport({
      host: "127.0.0.1",
      port: relay.port,
      secure: "tls",
      tlsRootCertificate: tlsFixture("ca.crt"),
    });

    try {
      const result = await transport.send({
        from: "a@example.test",
        to: ["b@example.test"],
        subject: "x",
        text: "y",
      });
      expect(result.status).toBe("accepted");
      expect(result.recipients).toEqual([{ address: "b@example.test", status: "accepted" }]);
    } finally {
      await transport.close();
      await relay.close();
    }
  });

  it.skipIf(!available)("fails closed without the added root", async () => {
    const relay = await fakeTlsRelay();
    const transport = createTransport({ host: "127.0.0.1", port: relay.port, secure: "tls" });

    try {
      const code = await errorCode(
        transport.send({
          from: "a@example.test",
          to: ["b@example.test"],
          subject: "x",
          text: "y",
        }),
      );
      expect(code).toBe("TLS_CERTIFICATE");
    } finally {
      await transport.close();
      await relay.close();
    }
  });

  it.skipIf(!available)("rejects a malformed trust anchor", () => {
    expect.assertions(1);
    try {
      createTransport({
        host: "127.0.0.1",
        port: 465,
        secure: "tls",
        tlsRootCertificate: "not a pem",
      });
    } catch (error) {
      expect((error as SendustError).code).toBe("VALIDATION_INVALID_OPTION");
    }
  });
});
