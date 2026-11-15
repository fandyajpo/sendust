// Forced-GC probe: proves the collector cannot close, corrupt, or destabilize a
// transport. There is no finalizer and no HTML-like weak reference in the
// binding, so dropping the last JavaScript reference only lets the wrapper be
// collected; the transport's own lifecycle stays explicit (`close()`).
//
// Run with: node --expose-gc tests/runtime/gc.mjs

import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const native = require("../../ts/sendust.node");

if (typeof globalThis.gc !== "function") {
  console.error("gc probe: run with `node --expose-gc tests/runtime/gc.mjs`");
  process.exit(1);
}

const checks = [];
const check = (name, actual, expected) => {
  checks.push({ name, actual, expected, ok: actual === expected });
};

const options = { host: "127.0.0.1", port: 1, secure: "tls" };

// 1. Fifty closed transports, dropped and collected: nothing to finalize, so the
// next transport must behave exactly like the first.
for (let index = 0; index < 50; index += 1) {
  const transport = new native.Transport(options);
  await transport.close();
}
globalThis.gc();

const afterCollection = new native.Transport(options);
check("state after collection", afterCollection.state(), "open");
await afterCollection.close();
check("close after collection", afterCollection.state(), "closed");

// 2. An open transport whose last reference is dropped is never closed by the
// collector: close is explicit and GC-independent.
{
  const abandoned = new native.Transport(options);
  check("abandoned state", abandoned.state(), "open");
}
globalThis.gc();

const later = new native.Transport(options);
check("a later transport is unaffected", later.state(), "open");
await later.close();
check("a later transport closes", later.state(), "closed");

const failed = checks.filter((entry) => !entry.ok);
if (failed.length > 0) {
  console.error(`gc probe: ${failed.length} check(s) failed`);
  for (const entry of failed) {
    console.error(`  ${entry.name}: expected ${entry.expected}, got ${entry.actual}`);
  }
  process.exit(1);
}

console.log(`Node ${process.version}: ${checks.length} forced-GC checks passed`);
