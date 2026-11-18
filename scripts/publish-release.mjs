#!/usr/bin/env node
/**
 * Publishes the verified release candidate: the five platform packages first,
 * then the facade.
 *
 * Nothing is published unless every tarball matches the hashes recorded for this
 * run and every packed binary matches the approved candidate in
 * `release/<version>/approved.json`. A package that already exists at this
 * version aborts the run instead of being replaced.
 *
 * Usage:
 *   node scripts/publish-release.mjs --tag next [--dry-run]
 *
 * Run in CI from the `release-candidate` artifact. A local machine normally holds
 * one target, which this script refuses; `--dry-run --allow-partial` only prints
 * the plan and does not claim completeness.
 */

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const shell = process.platform === "win32";

const tagIndex = process.argv.indexOf("--tag");
const tag = tagIndex === -1 ? null : process.argv[tagIndex + 1];
const dryRun = process.argv.includes("--dry-run");
const allowPartial = process.argv.includes("--allow-partial");

if (!tag) {
  console.error("publish-release: --tag <dist-tag> is required (for example --tag next)");
  process.exit(1);
}

const facade = JSON.parse(readFileSync(join(root, "ts", "package.json"), "utf8"));
const version = facade.version;
const release = join(root, "artifacts", "release", version);
const approved = JSON.parse(readFileSync(join(root, "release", version, "approved.json"), "utf8"));
const inventoryPath = join(release, "platform-packages.json");
const facadeTarball = join(root, "artifacts", "publish", `sendust-ts-${version}.tgz`);

if (approved.version !== version) {
  console.error(`publish-release: approved.json is for ${approved.version}, tree is ${version}`);
  process.exit(1);
}
if (!existsSync(inventoryPath)) {
  console.error("publish-release: no platform-packages.json; run `npm run pack:platform` first");
  process.exit(1);
}
if (!existsSync(facadeTarball)) {
  console.error("publish-release: no facade tarball; run `npm run prepare:publish` first");
  process.exit(1);
}

const sha256 = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");
const inventory = JSON.parse(readFileSync(inventoryPath, "utf8"));

if (inventory.version !== version) {
  console.error(`publish-release: inventory is for ${inventory.version}, tree is ${version}`);
  process.exit(1);
}

// The facade is target-independent and must reproduce the approved bytes.
const facadeSha = sha256(facadeTarball);
if (facadeSha !== approved.facade.sha256) {
  console.error(
    `publish-release: facade sha ${facadeSha} is not the approved ${approved.facade.sha256}`,
  );
  process.exit(1);
}

const targets = Object.keys(approved.artifacts);
const packed = new Map(inventory.platformPackages.map((entry) => [entry.target, entry]));
const missing = targets.filter((target) => !packed.has(target));

if (missing.length > 0 && !(dryRun && allowPartial)) {
  console.error(`publish-release: publication must be complete; missing ${missing.join(", ")}`);
  process.exit(1);
}

for (const [target, entry] of packed) {
  const file = join(release, entry.file);
  if (!existsSync(file)) {
    console.error(`publish-release: ${entry.file} is missing`);
    process.exit(1);
  }
  if (sha256(file) !== entry.sha256) {
    console.error(`publish-release: ${entry.file} does not match the inventory hash`);
    process.exit(1);
  }
  if (entry.artifactSha256 !== approved.artifacts[target]) {
    const message = `${entry.name} packs a binary that is not the approved ${target} artifact`;
    if (dryRun && allowPartial) {
      console.log(`publish-release: warning: ${message}`);
    } else {
      console.error(`publish-release: ${message}`);
      process.exit(1);
    }
  }
}

// A published version is immutable: refuse rather than attempt a replace.
function published(name) {
  try {
    execFileSync(npm, ["view", `${name}@${version}`, "version"], {
      cwd: root,
      stdio: "pipe",
      encoding: "utf8",
      shell,
    });
    return true;
  } catch {
    return false;
  }
}

const plan = [
  ...inventory.platformPackages.map((entry) => ({
    name: entry.name,
    file: join(release, entry.file),
  })),
  { name: approved.facade.name, file: facadeTarball },
];

for (const step of plan) {
  if (published(step.name)) {
    console.error(`publish-release: ${step.name}@${version} already exists; refusing to replace it`);
    process.exit(1);
  }
}

console.log(`publish-release: ${plan.length} package(s) at ${version}, dist-tag ${tag}`);
for (const step of plan) {
  console.log(`  ${step.name} <- ${step.file}`);
}

if (dryRun) {
  console.log("publish-release: dry run, nothing published");
  process.exit(0);
}

for (const step of plan) {
  console.log(`publish-release: publishing ${step.name}@${version}`);
  execFileSync(npm, ["publish", step.file, "--access", "public", "--tag", tag], {
    cwd: root,
    stdio: "inherit",
    shell,
  });
}

console.log(`publish-release: published ${plan.length} package(s) at ${version} with tag ${tag}`);
