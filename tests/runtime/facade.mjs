// Facade smoke: proves the built TypeScript package loads its native addon
// through `src/loader.ts` and keeps the public contract on Node.js, Bun, and
// Deno. Unlike `smoke.mjs` it never requires the addon directly.
//
// It needs no relay and no credentials: the one send case targets a closed
// loopback port, which exercises the async boundary, the core adapter, and the
// typed error mapping without trusting any external certificate.
//
// Run from the repository root:
//   node tests/runtime/facade.mjs
//   bun tests/runtime/facade.mjs
//   deno run --allow-ffi --allow-read --allow-net tests/runtime/facade.mjs
//
// `ts/dist` must be built (`npm --workspace ./ts run build`) and the addon
// staged as `ts/sendust.node` first.

import { createServer } from "node:net";
import { createTransport, version } from "../../ts/dist/index.js";

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

/** The facade exposes the stable code on the typed error itself. */
function codeOf(error) {
  const code = error && typeof error === "object" ? error.code : undefined;
  return typeof code === "string" ? code : String(error);
}

// 1. The public entry point resolves and exposes its version.
check("package version", typeof version, "string");

// 2. Invalid configuration maps to a stable code, thrown synchronously.
try {
  createTransport({ host: "", port: 465, secure: "tls" });
  check("invalid config", "no error", "CONFIG_INVALID_RELAY");
} catch (error) {
  check("invalid config", codeOf(error), "CONFIG_INVALID_RELAY");
}

// 3. Lifecycle: open, idempotent close, closed.
const transport = createTransport({ host: "127.0.0.1", port: 1, secure: "tls" });
check("initial state", transport.state, "open");
await transport.close();
await transport.close();
check("closed state", transport.state, "closed");

// 4. A send after close is refused without I/O.
try {
  await transport.send({ from: "a@example.test", to: ["b@example.test"], subject: "x", text: "y" });
  check("send after close", "no error", "TRANSPORT_CLOSED");
} catch (error) {
  check("send after close", codeOf(error), "TRANSPORT_CLOSED");
}

// 5. The async send path reaches the core: a relay that accepts and drops the
// socket fails the handshake with a typed error rather than hanging. This is
// deterministic on every runtime, unlike a closed port, whose refusal timing is
// OS-specific.
const server = createServer((socket) => socket.destroy());
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
const address = server.address();
const port = typeof address === "object" && address ? address.port : 1;

const live = createTransport({
  host: "127.0.0.1",
  port,
  secure: "tls",
  // Low budgets, in the shape the suite already validates, so a stall fails the
  // smoke instead of waiting out the production deadlines.
  timeouts: { connectMs: 1_000, operationMs: 1_000, totalMs: 1_000, closeMs: 1_000 },
});
try {
  await live.send({ from: "a@example.test", to: ["b@example.test"], subject: "x", text: "y" });
  check("send to a dropped relay", "no error", "TLS_HANDSHAKE");
} catch (error) {
  check("send to a dropped relay", codeOf(error), "TLS_HANDSHAKE");
}
await live.close();
await new Promise((resolve) => server.close(() => resolve()));

const failed = checks.filter((entry) => !entry.ok);
if (failed.length > 0) {
  console.error(`${runtime}: ${failed.length} facade check(s) failed`);
  for (const entry of failed) {
    console.error(`  ${entry.name}: expected ${entry.expected}, got ${entry.actual}`);
  }
  process.exit(1);
}

console.log(`${runtime}: ${checks.length} facade checks passed`);
