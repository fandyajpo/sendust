// Facade buffer-copy performance harness (the 10 MiB copy budget in
// `docs/qa/performance-budget.md`, run as release-checklist item 6).
//
// The adapter copies every content attachment before the native call starts:
//
//   content: new Uint8Array(attachment.content)
//
// so this probe times the synchronous part of `send()` — option and message
// mapping, the copy, and the native call's setup — through the built facade.
// The relay is a loopback socket that accepts and then stays silent, so the
// measured interval is local processing, never network latency.
//
// Not run in CI: shared runners are not a stable measurement environment.
//
// Prepare the package first:
//   npm run build:native
//   npm --workspace ./ts run build
//   node tests/perf/facade.mjs

import { createServer } from "node:net";

import { createTransport } from "../../ts/dist/index.js";

const WARMUP = 3;
const ITERATIONS = 20;
const MIB = 1024 * 1024;
const PAYLOAD_BYTES = 10 * MIB;

function summarize(samples) {
  const sorted = [...samples].sort((a, b) => a - b);
  const at = (quantile) => sorted[Math.round((sorted.length - 1) * quantile)];
  return { p50: at(0.5), p95: at(0.95), min: sorted[0], max: sorted[sorted.length - 1] };
}

function fail(message) {
  console.error(`facade perf: ${message}`);
  process.exit(1);
}

/** A relay that accepts connections and then stays silent (TLS handshake stall). */
async function silentServer(run) {
  const sockets = new Set();
  const server = createServer((socket) => {
    sockets.add(socket);
    socket.on("close", () => sockets.delete(socket));
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { port } = server.address();
  try {
    return await run(port);
  } finally {
    for (const socket of sockets) socket.destroy();
    await new Promise((resolve) => server.close(() => resolve()));
  }
}

const content = new Uint8Array(PAYLOAD_BYTES);
const message = {
  from: "sender@example.test",
  to: ["recipient@example.test"],
  subject: "Facade copy fixture",
  text: "y",
  attachments: [{ filename: "fixture.bin", content }],
};

const samples = [];

await silentServer(async (port) => {
  for (let index = 0; index < WARMUP + ITERATIONS; index += 1) {
    const transport = createTransport({
      host: "127.0.0.1",
      port,
      secure: "tls",
      timeouts: { connectMs: 1000, operationMs: 1000, totalMs: 1000, closeMs: 1000 },
    });

    // The copy and the mapping are synchronous, so they are timed before the
    // first await; the native send is aborted by the close that follows.
    const started = process.hrtime.bigint();
    const pending = transport.send(message).catch((error) => error);
    const micros = Number(process.hrtime.bigint() - started) / 1000;

    await transport.close();
    const outcome = await pending;

    // A refusal before the copy would make the measurement meaningless.
    if (outcome instanceof Error && /^(?:VALIDATION|LIMIT)_/.test(String(outcome.code))) {
      fail(`send() was refused before the copy: ${outcome.code}`);
    }
    if (index >= WARMUP) samples.push(micros);
  }
});

const stats = summarize(samples);
const round = (value) => value.toFixed(1);
console.log(
  `facade.send_copy_10mib, ${round(stats.p50)}, ${round(stats.p95)}, ${round(stats.min)}, ${round(stats.max)}, ${samples.length}`,
);

if (stats.p50 > 20_000 || stats.p95 > 40_000) {
  fail(
    `facade copy exceeds the 20 ms / 40 ms budget: p50 ${round(stats.p50)} us, p95 ${round(stats.p95)} us`,
  );
}

console.log(`Node ${process.version}: facade copy probe passed`);
