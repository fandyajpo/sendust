// Sendust vs Nodemailer, same loopback relay, same payloads, same concurrency.
//
// Every (client, workload) pair runs in its own process so RSS and CPU are not
// shared. Each worker warms up and runs three trials; the table reports the sum.
// No claim here is a delivery claim: both clients only report relay acceptance.

import { execFileSync } from "node:child_process";

// Sendust validates `maxConcurrent` to 1..32 for provider safety, so the
// comparison runs both clients at the same effective concurrency rather than
// claiming Sendust can exceed its own ceiling.
const workloads = [
  { name: "Sequential", messages: 100, concurrency: 1 },
  { name: "Low", messages: 100, concurrency: 5 },
  { name: "Medium", messages: 100, concurrency: 10 },
  { name: "High", messages: 100, concurrency: 25 },
  { name: "Ceiling", messages: 100, concurrency: 32 },
  { name: "Extended", messages: 1000, concurrency: 32 },
];

const clients = ["sendust", "nodemailer"];
const results = [];

for (const workload of workloads) {
  // Alternate which client runs first, so a warm page cache cannot favour one.
  const order = workload.messages % 200 === 0 ? [...clients] : [...clients].reverse();
  for (const client of order) {
    const output = execFileSync(
      process.execPath,
      ["bench/worker.mjs", client, String(workload.messages), String(workload.concurrency)],
      { encoding: "utf8", cwd: new URL("..", import.meta.url).pathname },
    );
    const result = JSON.parse(output.trim().split("\n").pop());
    results.push({ workload: workload.name, ...result });
    console.log(
      `${workload.name.padEnd(10)} ${client.padEnd(11)} ${String(result.messages).padStart(5)} msg  ` +
        `conc ${String(result.concurrency).padStart(3)}  ${result.throughput.toFixed(1).padStart(7)}/s  ` +
        `p50 ${result.latency.p50.toFixed(1).padStart(6)}ms  conns ${String(result.connections).padStart(3)}  ` +
        `ok ${result.ok} fail ${result.failed}`,
    );
  }
}

const round = (value, digits = 1) => Number(value.toFixed(digits));
const byKey = new Map();
for (const result of results) {
  byKey.set(`${result.workload}:${result.client}`, result);
}

console.log("\n| Workload | Concurrency | Messages | Sendust msg/s | Nodemailer msg/s | Sendust p95 | Nodemailer p95 | Sendust conns | Nodemailer conns |");
console.log("| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |");
for (const workload of workloads) {
  const send = byKey.get(`${workload.name}:sendust`);
  const node = byKey.get(`${workload.name}:nodemailer`);
  if (!send || !node) continue;
  console.log(
    `| ${workload.name} | ${workload.concurrency} | ${workload.messages} | ` +
      `${round(send.throughput)} | ${round(node.throughput)} | ` +
      `${round(send.latency.p95)} ms | ${round(node.latency.p95)} ms | ` +
      `${send.connections} | ${node.connections} |`,
  );
}

const failures = results.filter((result) => result.failed > 0);
if (failures.length > 0) {
  console.error(`\n${failures.length} run(s) had failures:`);
  for (const result of failures) {
    console.error(`  ${result.client} ${result.workload}: ${JSON.stringify(result.errors)}`);
  }
  process.exitCode = 1;
}
