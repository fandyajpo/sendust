// Executable companion to `docs/guides/quick-start.md`.
//
// It runs the guide's loopback example: a TLS relay with the committed fixture
// certificate, the additive trust anchor, one send, an explicit close, and the
// refusal that follows. The guide's snippets are this file's shape, so a doc
// change that breaks the sample fails here.
//
// Run from the repository root after `npm run build --workspace ts`:
//   node tests/docs/quickstart.mjs

import { readFileSync } from "node:fs";
import { createServer } from "node:tls";
import { fileURLToPath } from "node:url";
import { createTransport } from "../../ts/dist/index.js";

const fixture = (name) =>
  readFileSync(fileURLToPath(new URL(`../fixtures/tls/${name}`, import.meta.url)), "utf8");

/** A loopback SMTP relay over TLS that accepts the message (`250` at DATA). */
async function fakeRelay() {
  const sockets = new Set();
  const server = createServer(
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

const relay = await fakeRelay();

const transport = createTransport({
  host: "127.0.0.1",
  port: relay.port,
  secure: "tls",
  // Test only: the relay presents the committed fixture CA. Production code
  // trusts the public roots and never passes this option.
  tlsRootCertificate: fixture("ca.crt"),
});

try {
  const result = await transport.send({
    from: "sender@example.test",
    to: ["recipient@example.test"],
    subject: "Hello",
    text: "Hello from Sendust.",
  });

  if (result.status !== "accepted") {
    throw new Error(`expected accepted, got ${result.status}`);
  }
  console.log(`quick start: ${result.status} for ${result.recipients.length} recipient(s)`);
} finally {
  await transport.close();
}

let afterClose = null;
try {
  await transport.send({
    from: "sender@example.test",
    to: ["recipient@example.test"],
    subject: "x",
    text: "y",
  });
} catch (error) {
  afterClose = error?.code ?? String(error);
}

await relay.close();

if (afterClose !== "TRANSPORT_CLOSED") {
  throw new Error(`expected TRANSPORT_CLOSED after close, got ${afterClose}`);
}

console.log("quick start: closed, and a send after close is TRANSPORT_CLOSED");
