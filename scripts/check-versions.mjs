#!/usr/bin/env node
/**
 * Version consistency guard.
 *
 * One release version is repeated in files each ecosystem needs, plus the
 * changelog. This fails when any of them disagree, when the lockfile records a
 * different crate version, or when the changelog has no entry for the version.
 *
 * Usage: node scripts/check-versions.mjs
 */

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("..", import.meta.url));
const read = (path) => readFileSync(join(ROOT, path), "utf8");
const json = (path) => JSON.parse(read(path));

const failures = [];
const fail = (message) => failures.push(message);

const rootPackage = json("package.json");
const version = rootPackage.version;

const checks = [
  ["package.json version", version],
  ["ts/package.json version", json("ts/package.json").version],
];

const cargo = /\[workspace\.package\]([\s\S]*?)\n\[/.exec(read("Cargo.toml"))?.[1] ?? "";
checks.push(["Cargo.toml workspace version", /version = "([^"]+)"/.exec(cargo)?.[1] ?? "missing"]);

checks.push([
  "ts/src/index.ts version constant",
  /export const version = "([^"]+)"/.exec(read("ts/src/index.ts"))?.[1] ?? "missing",
]);

const lock = read("Cargo.lock");
for (const name of ["sendust-core", "sendust-napi"]) {
  const recorded = new RegExp(`name = "${name}"\\nversion = "([^"]+)"`).exec(lock)?.[1] ?? "missing";
  checks.push([`Cargo.lock ${name} version`, recorded]);
}

for (const [label, actual] of checks) {
  if (actual !== version) {
    fail(`${label} is ${actual}, but package.json says ${version}`);
  }
}

const changelog = read("CHANGELOG.md");
if (!changelog.includes(`## [${version}]`)) {
  fail(`CHANGELOG.md has no "## [${version}]" section`);
}

if (failures.length > 0) {
  for (const message of failures) {
    console.error(`FAIL: ${message}`);
  }
  process.exit(1);
}
console.log(`PASS: version ${version} agrees across the manifests, the lockfile, the facade, and the changelog`);
