// Lifecycle performance harness (P13-004): BUSY rejection, close, and the
// no-queue invariant, measured through the real Node-API + core path.
//
// Run after `npm run build:native`:
//   node tests/perf/lifecycle.mjs

import { readdirSync } from "node:fs";
import { createRequire } from "node:module";
import { createServer } from "node:net";

const require = createRequire(import.meta.url);
const native = require("../../ts/sendust.node");

const WARMUP = 3;
const ITERATIONS = 20;
const CLOSE_CYCLES = 1000;

const message = { from: "a@example.test", to: ["b@example.test"], subject: "x", text: "y" };

function codeOf(error) {
  const text = String(error && error.message ? error.message : error);
  const index = text.indexOf("SENDUST:");
  if (index === -1) return text;
  return JSON.parse(text.slice(index + 8)).code;
}

function summarize(samples) {
  const sorted = [...samples].sort((a, b) => a - b);
  const at = (q) => sorted[Math.round((sorted.length - 1) * q)];
  return { p50: at(0.5), p95: at(0.95), min: sorted[0], max: sorted[sorted.length - 1] };
}

function report(metric, samples) {
  const stats = summarize(samples);
  const round = (value) => value.toFixed(1);
  console.log(
    `${metric}, ${round(stats.p50)}, ${round(stats.p95)}, ${round(stats.min)}, ${round(stats.max)}, ${samples.length}`,
  );
  return stats;
}

function fail(message) {
  console.error(`lifecycle probe: ${message}`);
  process.exit(1);
}

function descriptors() {
  return readdirSync("/dev/fd").length;
}

/** A relay that accepts connections and then stays silent (TLS handshake stall). */
async function silentServer(run) {
  const sockets = new Set();
  const waiters = [];
  let connections = 0;
  const server = createServer((socket) => {
    connections += 1;
    sockets.add(socket);
    socket.on("close", () => sockets.delete(socket));
    for (const resolve of waiters.splice(0)) resolve();
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const port = server.address().port;
  try {
    return await run({
      port,
      connections: () => connections,
      waitForConnection: () => new Promise((resolve) => waiters.push(resolve)),
    });
  } finally {
    for (const socket of sockets) socket.destroy();
    await new Promise((resolve) => server.close(() => resolve()));
  }
}

const closeIdle = [];
const busy = [];
const closeDuringSend = [];

await silentServer(async ({ port, connections, waitForConnection }) => {
  // 1. BUSY rejection while one send is in flight: immediate, and no dial.
  for (let index = 0; index < WARMUP + ITERATIONS; index += 1) {
    const transport = new native.Transport({ host: "127.0.0.1", port, secure: "tls" });
    const inFlight = transport.send(message).catch((error) => error);
    await waitForConnection();

    const started = process.hrtime.bigint();
    const rejected = await transport.send(message).catch((error) => error);
    const micros = Number(process.hrtime.bigint() - started) / 1000;

    if (codeOf(rejected) !== "TRANSPORT_BUSY") fail(`expected TRANSPORT_BUSY, got ${codeOf(rejected)}`);
    await transport.close();
    await inFlight;
    if (index >= WARMUP) busy.push(micros);
  }
  if (connections() !== WARMUP + ITERATIONS) {
    fail(`BUSY must not dial: ${connections()} connections for ${WARMUP + ITERATIONS} sends`);
  }

  // 2. Close during a stalled send stays inside the close budget.
  for (let index = 0; index < WARMUP + ITERATIONS; index += 1) {
    const transport = new native.Transport({
      host: "127.0.0.1",
      port,
      secure: "tls",
      timeouts: { connectMs: 1000, operationMs: 1000, totalMs: 1000, closeMs: 1000 },
    });
    const inFlight = transport.send(message).catch((error) => error);
    await waitForConnection();

    const started = process.hrtime.bigint();
    await transport.close();
    const micros = Number(process.hrtime.bigint() - started) / 1000;
    await inFlight;
    if (index >= WARMUP) closeDuringSend.push(micros);
  }

  // 3. No queue under concurrent load: every extra call is BUSY, one connection.
  const transport = new native.Transport({ host: "127.0.0.1", port, secure: "tls" });
  const before = connections();
  const inFlight = transport.send(message).catch((error) => error);
  await waitForConnection();
  const concurrent = await Promise.all(
    Array.from({ length: 50 }, () => transport.send(message).catch((error) => codeOf(error))),
  );
  if (!concurrent.every((code) => code === "TRANSPORT_BUSY")) {
    fail(`concurrent sends must all be BUSY, saw ${JSON.stringify(concurrent)}`);
  }
  if (connections() - before !== 1) {
    fail(`concurrent load opened ${connections() - before} connections instead of 1`);
  }
  await transport.close();
  await inFlight;
});

// 4. Idle close and repeated open/close cycles.
const baselineDescriptors = descriptors();
await silentServer(async ({ port }) => {
  for (let index = 0; index < CLOSE_CYCLES; index += 1) {
    const transport = new native.Transport({ host: "127.0.0.1", port, secure: "tls" });
    const started = process.hrtime.bigint();
    await transport.close();
    closeIdle.push(Number(process.hrtime.bigint() - started) / 1000);
  }
});
const descriptorDelta = descriptors() - baselineDescriptors;

const busyStats = report("busy.rejection", busy);
const closeStats = report("close.idle", closeIdle.slice(0, ITERATIONS));
const closeSendStats = report("close.during_send", closeDuringSend);
console.log(`close.cycles, ${CLOSE_CYCLES}, descriptor_delta, ${descriptorDelta}`);

if (busyStats.p50 > 100 || busyStats.p95 > 500) fail("BUSY rejection exceeds 0.1 ms / 0.5 ms");
if (closeStats.p50 > 5000 || closeStats.p95 > 10_000) fail("idle close exceeds 5 ms / 10 ms");
if (closeSendStats.p50 > 1_100_000) fail("close during send exceeds the 1 s budget + 100 ms");
if (Math.abs(descriptorDelta) > 2) fail(`descriptor count drifted by ${descriptorDelta}`);

console.log(`Node ${process.version}: lifecycle probe passed`);
