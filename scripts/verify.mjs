#!/usr/bin/env node
/**
 * One-command local verification: build the addon, build the package, run the
 * TypeScript suite, build the per-platform artifact, and load it.
 *
 * Usage: npm run verify
 */

import { execFileSync } from "node:child_process";
import { readdirSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const npm = process.platform === "win32" ? "npm.cmd" : "npm";

function step(name, command, args, cwd = root) {
  console.log(`\n== ${name}`);
  // Node refuses to spawn a `.cmd` without a shell on Windows (CVE-2024-27910).
  execFileSync(command, args, {
    cwd,
    stdio: "inherit",
    shell: process.platform === "win32",
  });
}

step("native addon", "node", ["scripts/build-addon.mjs"]);
step("package build", npm, ["run", "build"], join(root, "ts"));
step("typescript tests", npm, ["test"], join(root, "ts"));
step("platform artifact", "node", ["scripts/build-artifact.mjs"]);
step("per-platform resolution", "node", ["scripts/verify-platform.mjs"]);

const artifacts = join(root, "artifacts");
const artifact = readdirSync(artifacts).find((name) => name.endsWith(".node"));
if (!artifact) {
  console.error("\nverify: no artifact was produced");
  process.exit(1);
}

const binding = createRequire(import.meta.url)(join(artifacts, artifact));
if (!binding.Transport) {
  console.error(`\nverify: ${artifact} has no Transport export`);
  process.exit(1);
}

console.log(`\nverify: OK — ${artifact} loads`);
