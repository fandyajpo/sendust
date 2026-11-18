#!/usr/bin/env node
/**
 * Publishes the verified release candidate: the five platform packages first,
 * then the facade.
 *
 * The bundle must be a CI build of the approved commit: `provenance.json` names
 * that commit, every tarball matches its own inventory hash, every packed binary
 * is the bundle's artifact according to `SHA256SUMS`, and every target listed as
 * byte-reproducible in `release/<version>/approved.json` must also match the
 * approved hash exactly. The facade and the Linux/macOS binaries reproduce byte
 * for byte in CI, so they are compared exactly; the Windows DLL does not (see
 * "What is not claimed" in docs/engineering/reproducible-build.md), so it is
 * held to the approved commit and the bundle's own checksums instead of a frozen
 * hash from another run.
 *
 * Nothing is published unless all of that holds. A package that already exists
 * at this version aborts the run instead of being replaced.
 *
 * Usage:
 *   node scripts/publish-release.mjs --tag next [--dry-run] [--allow-partial]
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
const provenancePath = join(release, "provenance.json");
const sumsPath = join(release, "SHA256SUMS");
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

// The bundle must be a build of the approved commit, not of whatever happens to
// be checked out. `provenance.json` is written by `assemble-rc` in the bundle.
if (!existsSync(provenancePath)) {
  if (dryRun) {
    console.log("publish-release: warning: no provenance.json, so the approved commit is unverified");
  } else {
    console.error("publish-release: no provenance.json; run `npm run assemble:rc` first");
    process.exit(1);
  }
} else {
  const provenance = JSON.parse(readFileSync(provenancePath, "utf8"));
  if (provenance.version !== version) {
    console.error(`publish-release: provenance is for ${provenance.version}, tree is ${version}`);
    process.exit(1);
  }
  if (provenance.commit !== approved.commit) {
    const message = `the bundle was built from ${provenance.commit}, not the approved ${approved.commit}`;
    if (dryRun) {
      console.log(`publish-release: warning: ${message}`);
    } else {
      console.error(`publish-release: ${message}`);
      process.exit(1);
    }
  }
  if (provenance.dirty) {
    const message = "the bundle was assembled from a dirty working tree";
    if (dryRun) {
      console.log(`publish-release: warning: ${message}`);
    } else {
      console.error(`publish-release: ${message}`);
      process.exit(1);
    }
  }
}

// The bundle's own checksums: a packed binary must be the artifact it built.
const sums = new Map();
if (existsSync(sumsPath)) {
  for (const line of readFileSync(sumsPath, "utf8").split("\n")) {
    const match = /^([0-9a-f]{64})\s+(.+)$/.exec(line.trim());
    if (match) {
      sums.set(match[2], match[1]);
    }
  }
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

// Targets whose bytes are proven identical across CI runs are compared to the
// approved hash; the rest are identified by the approved commit. Defaulting to
// every target keeps an older approved.json strict rather than lenient.
const byteReproducible = new Set(approved.byteReproducible ?? targets);

for (const [target, entry] of packed) {
  if (!targets.includes(target)) {
    console.error(`publish-release: ${target} is not in the approved artifact set`);
    process.exit(1);
  }
  const file = join(release, entry.file);
  if (!existsSync(file)) {
    console.error(`publish-release: ${entry.file} is missing`);
    process.exit(1);
  }
  if (sha256(file) !== entry.sha256) {
    console.error(`publish-release: ${entry.file} does not match the inventory hash`);
    process.exit(1);
  }
  if (sums.size > 0) {
    const built = sums.get(`sendust-${target}.node`);
    if (built !== entry.artifactSha256) {
      console.error(
        `publish-release: ${entry.name} packs ${entry.artifactSha256}, but the bundle's checksums record ${built ?? "nothing"} for ${target}`,
      );
      process.exit(1);
    }
  }

  const approvedHash = approved.artifacts[target];
  if (entry.artifactSha256 === approvedHash) {
    continue;
  }
  const message = `${entry.name} packs ${entry.artifactSha256}, but the approved ${target} artifact is ${approvedHash}`;
  if (!byteReproducible.has(target)) {
    console.log(
      `publish-release: ${entry.name} is a fresh CI build of the approved commit; ${target} does not reproduce byte for byte (observed ${entry.artifactSha256}, approval run ${approvedHash})`,
    );
    continue;
  }
  if (dryRun && allowPartial) {
    console.log(`publish-release: warning: ${message}`);
    continue;
  }
  console.error(`publish-release: ${message}`);
  process.exit(1);
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
for (const [target, entry] of packed) {
  console.log(`  ${target} artifact ${entry.artifactSha256}`);
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
