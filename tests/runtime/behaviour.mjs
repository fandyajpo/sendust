// Behaviour harness: the facade's outcome contract on Node.js, Bun, and Deno.
//
// Runs the built package against loopback relays: accepted (`250`), rejected
// (`451`), unknown (the relay drops the final reply), BUSY (a second send while
// one is in flight), close, the attachment cap, and TLS failing closed without
// the added root. No network beyond loopback, no credentials, no Vitest.
//
// Run from the repository root after `npm --workspace ./ts run build`:
//   node tests/runtime/behaviour.mjs
//   bun tests/runtime/behaviour.mjs
//   deno run --allow-ffi --allow-read --allow-net tests/runtime/behaviour.mjs

import { readFileSync } from "node:fs";
import { createServer as createTcpServer } from "node:net";
import { createServer as createTlsServer } from "node:tls";
import { fileURLToPath } from "node:url";
import { createTransport } from "../../ts/dist/index.js";

const runtime =
  typeof globalThis.Deno !== "undefined"
    ? `Deno ${globalThis.Deno.version.deno}`
    : typeof globalThis.Bun !== "undefined"
      ? `Bun ${globalThis.Bun.version}`
      : `Node ${process.version}`;

const checks = [];
const check = (name, actual, expected) => {
  checks.push({ name, actual, expected, ok: actual === expected });
};

/** The stable code the facade exposes on a typed error. */
function codeOf(error) {
  const code = error && typeof error === "object" ? error.code : undefined;
  return typeof code === "string" ? code : String(error);
}

function fixture(name) {
  return readFileSync(fileURLToPath(new URL(`../fixtures/tls/${name}`, import.meta.url)), "utf8");
}

const trust = fixture("ca.crt");
const message = { from: "a@example.test", to: ["b@example.test"], subject: "x", text: "y" };

/**
 * A loopback SMTP relay over TLS with the committed fixture certificate.
 * `final` decides the reply to the body terminator: `250`, `451`, or a dropped
 * connection, which the contract reports as `unknown`.
 */
async function relay(final = "accepted") {
  const sockets = new Set();
  const server = createTlsServer(
    { key: fixture("valid-server.key"), cert: fixture("valid-server.crt") },
    (socket) => {
      sockets.add(socket);
      socket.on("close", () => sockets.delete(socket));
      socket.setEncoding("utf8");
      socket.write("220 sendust.test ESMTP ready\r\n");
      let buffer = "";
      let inData = false;
      socket.on("data", (chunk) => {
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
              if (final === "drop") {
                socket.destroy();
                return;
              }
              socket.write(
                final === "rejected" ? "451 4.3.0 try again later\r\n" : "250 2.0.0 accepted\r\n",
              );
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
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  return {
    port: typeof address === "object" && address ? address.port : 1,
    close: async () => {
      for (const socket of sockets) {
        socket.destroy();
      }
      await new Promise((resolve) => server.close(() => resolve()));
    },
  };
}

// 1. Accepted: the relay takes responsibility; nothing is claimed about delivery.
{
  const live = await relay();
  const transport = createTransport({
    host: "127.0.0.1",
    port: live.port,
    secure: "tls",
    tlsRootCertificate: trust,
  });
  try {
    const result = await transport.send(message);
    check("accepted status", result.status, "accepted");
    check("accepted recipient", result.recipients[0]?.status, "accepted");
  } finally {
    await transport.close();
    await live.close();
  }
}

// 2. Rejected: an explicit negative final reply is an outcome, not an error.
{
  const live = await relay("rejected");
  const transport = createTransport({
    host: "127.0.0.1",
    port: live.port,
    secure: "tls",
    tlsRootCertificate: trust,
  });
  try {
    const result = await transport.send(message);
    check("rejected status", result.status, "rejected");
    check("rejected recipient", result.recipients[0]?.status, "rejected");
    check("rejected response", result.response?.code, 451);
  } finally {
    await transport.close();
    await live.close();
  }
}

// 3. Unknown: the final reply never arrives, so the disposition is ambiguous.
{
  const live = await relay("drop");
  const transport = createTransport({
    host: "127.0.0.1",
    port: live.port,
    secure: "tls",
    tlsRootCertificate: trust,
  });
  try {
    const result = await transport.send(message);
    check("unknown status", result.status, "unknown");
    check("unknown recipient", result.recipients[0]?.status, "unknown");
    check("unknown response", result.response, undefined);
  } finally {
    await transport.close();
    await live.close();
  }
}

// 4. BUSY: a second send never dials, and close aborts the one in flight.
{
  let connections = 0;
  const sockets = new Set();
  let open;
  const firstConnection = new Promise((resolve) => {
    open = resolve;
  });
  const server = createTcpServer((socket) => {
    connections += 1;
    sockets.add(socket);
    socket.on("close", () => sockets.delete(socket));
    open?.();
    open = undefined;
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  const transport = createTransport({
    host: "127.0.0.1",
    port: typeof address === "object" && address ? address.port : 1,
    secure: "tls",
  });
  const sending = transport.send(message).then(
    () => null,
    (error) => error,
  );
  await firstConnection;
  let busy = null;
  try {
    await transport.send(message);
  } catch (error) {
    busy = codeOf(error);
  }
  check("busy", busy, "TRANSPORT_BUSY");
  check("busy connections", connections, 1);
  await transport.close();
  check("close aborts the send", codeOf(await sending), "TRANSPORT_CLOSED");
  for (const socket of sockets) {
    socket.destroy();
  }
  await new Promise((resolve) => server.close(() => resolve()));
}

// 5. Close: idempotent, reaches `closed`, and refuses later sends.
{
  const transport = createTransport({ host: "127.0.0.1", port: 1, secure: "tls" });
  check("open state", transport.state, "open");
  await transport.close();
  await transport.close();
  check("closed state", transport.state, "closed");
  let code = null;
  try {
    await transport.send(message);
  } catch (error) {
    code = codeOf(error);
  }
  check("send after close", code, "TRANSPORT_CLOSED");
}

// 6. Limits: the per-attachment cap is enforced before any I/O.
{
  const transport = createTransport({
    host: "127.0.0.1",
    port: 1,
    secure: "tls",
    limits: { maxAttachmentBytes: 4 },
  });
  let code = null;
  try {
    await transport.send({
      ...message,
      attachments: [{ filename: "five.bin", content: new Uint8Array(5) }],
    });
  } catch (error) {
    code = codeOf(error);
  }
  await transport.close();
  check("attachment cap", code, "LIMIT_ATTACHMENT_BYTES");
}

// 7. TLS: verification cannot be relaxed, so an unlisted root fails closed.
{
  const live = await relay();
  const transport = createTransport({ host: "127.0.0.1", port: live.port, secure: "tls" });
  let code = null;
  try {
    await transport.send(message);
  } catch (error) {
    code = codeOf(error);
  }
  await transport.close();
  await live.close();
  check("tls fails closed", code, "TLS_CERTIFICATE");
}

const failed = checks.filter((entry) => !entry.ok);
if (failed.length > 0) {
  console.error(`${runtime}: ${failed.length} behaviour check(s) failed`);
  for (const entry of failed) {
    console.error(`  ${entry.name}: expected ${entry.expected}, got ${entry.actual}`);
  }
  process.exit(1);
}

console.log(`${runtime}: ${checks.length} behaviour checks passed`);
