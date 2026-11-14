// Runtime smoke: proves the shared Node-API addon loads and preserves the
// async lifecycle contract on Node.js, Bun, and Deno.
//
// It needs no relay and no credentials: the send case targets a closed loopback
// port, which exercises the async boundary, the core adapter, and the error
// mapping without trusting any external certificate.

import { createServer } from "node:net";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const native = require("../../ts/sendust.node");

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

function codeOf(error) {
  const message = String(error && error.message ? error.message : error);
  const index = message.indexOf("SENDUST:");
  if (index === -1) return message;
  return JSON.parse(message.slice(index + 8)).code;
}

// 1. Invalid configuration maps to a stable code.
try {
  new native.Transport({ host: "", port: 465, secure: "tls" });
  check("invalid config", "no error", "CONFIG_INVALID_RELAY");
} catch (error) {
  check("invalid config", codeOf(error), "CONFIG_INVALID_RELAY");
}

// 2. Lifecycle: open, idempotent close, closed.
const transport = new native.Transport({ host: "127.0.0.1", port: 1, secure: "tls" });
check("initial state", transport.state(), "open");
await transport.close();
await transport.close();
check("closed state", transport.state(), "closed");

// 3. Send after close is refused without I/O.
try {
  await transport.send({ from: "a@example.test", to: ["b@example.test"], subject: "x", text: "y" });
  check("send after close", "no error", "TRANSPORT_CLOSED");
} catch (error) {
  check("send after close", codeOf(error), "TRANSPORT_CLOSED");
}

// 4. The async send path runs through the core: a closed port fails as a
// protocol error rather than hanging or throwing an internal error.
const server = createServer();
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const address = server.address();
const port = typeof address === "object" && address ? address.port : 1;
await new Promise((resolve) => server.close(() => resolve()));

const live = new native.Transport({ host: "127.0.0.1", port, secure: "tls" });
try {
  await live.send({ from: "a@example.test", to: ["b@example.test"], subject: "x", text: "y" });
  check("send to closed port", "no error", "PROTOCOL_CONNECTION_RESET");
} catch (error) {
  check("send to closed port", codeOf(error), "PROTOCOL_CONNECTION_RESET");
}
await live.close();

const failed = checks.filter((entry) => !entry.ok);
if (failed.length > 0) {
  console.error(`${runtime}: ${failed.length} check(s) failed`);
  for (const entry of failed) {
    console.error(`  ${entry.name}: expected ${entry.expected}, got ${entry.actual}`);
  }
  process.exit(1);
}

console.log(`${runtime}: ${checks.length} binding checks passed`);
