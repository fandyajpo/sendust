import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { createServer, type Socket } from "node:net";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { describe, expect, it } from "vitest";
import { SendustError, createTransport, version, type Attachment } from "../src/index.ts";

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
    const sockets = new Set<Socket>();
    const server = createServer((socket) => {
      sockets.add(socket);
      socket.on("close", () => sockets.delete(socket));
    });
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

    // Let the delivery start and reach the handshake before close races it.
    await new Promise((resolve) => setTimeout(resolve, 100));
    await transport.close();

    const error = await sending;

    expect(error).toBeInstanceOf(SendustError);
    expect((error as SendustError).code).toBe("TRANSPORT_CLOSED");
    expect(transport.state).toBe("closed");

    for (const socket of sockets) {
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
