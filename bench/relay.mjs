// A loopback SMTP relay for benchmarks: TLS with the committed test fixture,
// one command loop per connection, no AUTH. It answers RSET, so both clients
// can reuse a session exactly as they would against a real relay.

import { readFileSync } from "node:fs";
import { createServer } from "node:tls";

const root = new URL("..", import.meta.url).pathname;

function fixture(name) {
  return readFileSync(`${root}tests/fixtures/tls/${name}`, "utf8");
}

export async function startRelay() {
  const sockets = new Set();
  let connections = 0;
  let messages = 0;

  const server = createServer(
    { key: fixture("valid-server.key"), cert: fixture("valid-server.crt") },
    (socket) => {
      connections += 1;
      sockets.add(socket);
      socket.on("close", () => sockets.delete(socket));
      socket.setEncoding("utf8");
      socket.write("220 bench.test ESMTP ready\r\n");

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
              messages += 1;
              socket.write("250 2.0.0 accepted\r\n");
            }
            continue;
          }

          if (line.startsWith("EHLO")) {
            socket.write("250-bench.test\r\n250 8BITMIME\r\n");
          } else if (line.startsWith("MAIL")) {
            socket.write("250 2.1.0 ok\r\n");
          } else if (line.startsWith("RCPT")) {
            socket.write("250 2.1.5 ok\r\n");
          } else if (line === "DATA") {
            inData = true;
            socket.write("354 go\r\n");
          } else if (line.startsWith("RSET") || line.startsWith("NOOP")) {
            socket.write("250 2.0.0 ok\r\n");
          } else if (line.startsWith("QUIT")) {
            socket.write("221 2.0.0 bye\r\n");
            socket.end();
          } else {
            socket.write("500 5.5.2 unrecognized\r\n");
          }
        }
      });
    },
  );

  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  const port = typeof address === "object" && address ? address.port : 1;

  return {
    port,
    trustAnchor: fixture("ca.crt"),
    connections: () => connections,
    messages: () => messages,
    close: async () => {
      for (const socket of sockets) {
        socket.destroy();
      }
      await new Promise((resolve) => server.close(() => resolve()));
    },
  };
}
