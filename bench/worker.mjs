// One benchmark run in its own process, so CPU and RSS belong to one client.
//
// Usage: node bench/worker.mjs <sendust|nodemailer> <messages> <concurrency>
// Prints one JSON line: counts, duration, per-message latencies, connections.

import { performance } from "node:perf_hooks";
import { createTransport } from "../ts/dist/index.js";
import { startRelay } from "./relay.mjs";

const [client, messagesArg, concurrencyArg] = process.argv.slice(2);
const messages = Number(messagesArg);
const concurrency = Number(concurrencyArg);

const message = (index) => ({
  from: "sender@bench.test",
  to: ["recipient@bench.test"],
  subject: `benchmark ${index}`,
  text: `Hello from the Sendust benchmark, message ${index}.`,
});

/** Sendust: one transport with the delivery engine's concurrency fixed. */
async function sendustSender(relay) {
  const transport = createTransport({
    host: "127.0.0.1",
    port: relay.port,
    secure: "tls",
    tlsRootCertificate: relay.trustAnchor,
    delivery: {
      maxConcurrent: concurrency,
      maxConnections: concurrency,
      // Lift the per-session cap so both clients are compared on connection
      // reuse alone; the product default stays 100.
      maxMessagesPerSession: 10_000,
    },
  });
  return {
    send: (index) => transport.send(message(index)),
    close: () => transport.close(),
    stats: () => transport.stats,
  };
}

/** Nodemailer: a pooled transport with the same connection ceiling. */
async function nodemailerSender(relay) {
  const { default: nodemailer } = await import("nodemailer");
  const transporter = nodemailer.createTransport({
    host: "127.0.0.1",
    port: relay.port,
    secure: true,
    tls: { ca: [relay.trustAnchor] },
    pool: true,
    maxConnections: concurrency,
    maxMessages: Infinity,
  });
  return {
    send: (index) => transporter.sendMail(message(index)),
    close: () => transporter.close(),
    stats: () => ({}),
  };
}

async function runBatch(sender, count, latencies) {
  let ok = 0;
  let failed = 0;
  const errors = new Map();
  await Promise.all(
    Array.from({ length: count }, async (_, index) => {
      const started = performance.now();
      try {
        await sender.send(index);
        ok += 1;
      } catch (error) {
        failed += 1;
        const code = error?.code ?? error?.name ?? "unknown";
        errors.set(code, (errors.get(code) ?? 0) + 1);
      }
      latencies.push(performance.now() - started);
    }),
  );
  return { ok, failed, errors: Object.fromEntries(errors) };
}

const relay = await startRelay();
const sender = client === "nodemailer" ? await nodemailerSender(relay) : await sendustSender(relay);

// Warm-up: one small batch, so the first trial does not pay JIT and TLS setup.
const warmup = Math.min(20, messages);
await runBatch(sender, warmup, []);

const latencies = [];
const trials = 3;
let ok = 0;
let failed = 0;
let durationMs = 0;
const errors = {};

for (let trial = 0; trial < trials; trial += 1) {
  const started = performance.now();
  const result = await runBatch(sender, messages, latencies);
  durationMs += performance.now() - started;
  ok += result.ok;
  failed += result.failed;
  for (const [code, count] of Object.entries(result.errors)) {
    errors[code] = (errors[code] ?? 0) + count;
  }
}

const stats = sender.stats();
const connections = relay.connections();
await sender.close();
await relay.close();

const sorted = latencies.sort((a, b) => a - b);
const percentile = (p) => {
  if (sorted.length === 0) return 0;
  const index = Math.min(sorted.length - 1, Math.ceil((p / 100) * sorted.length) - 1);
  return sorted[index];
};

process.stdout.write(
  `${JSON.stringify({
    client,
    messages,
    concurrency,
    trials,
    ok,
    failed,
    errors,
    durationMs: Math.round(durationMs),
    throughput: ok / (durationMs / 1000),
    latency: {
      avg: latencies.reduce((total, value) => total + value, 0) / (latencies.length || 1),
      p50: percentile(50),
      p95: percentile(95),
      p99: percentile(99),
      max: sorted.length > 0 ? sorted[sorted.length - 1] : 0,
    },
    connections,
    messagesDelivered: relay.messages(),
    reuse: stats,
    rssBytes: process.memoryUsage().rss,
  })}\n`,
);
