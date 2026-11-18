#!/usr/bin/env node
/**
 * Packs the per-platform native packages from the built artifacts.
 *
 * Each package contains only `sendust.node` for its target, plus the `os`, `cpu`
 * and `libc` fields that make npm skip it on every other machine (ADR-0020).
 * The version is read from `ts/package.json`, so a platform package can never
 * disagree with the facade, and the package list comes from the loader's own
 * map, so it can never name a package the facade does not resolve.
 *
 * Usage:
 *   node scripts/pack-platform.mjs [--require-all] [--allow-partial]
 *
 * Artifacts are read from `artifacts/sendust-<target>.node` (the CI artifacts
 * job uploads one per target). Tarballs and their hashes are written to
 * `artifacts/release/<version>/packages/`, and `platform-packages.json` plus
 * `PLATFORM-PACKAGES.sha256` record the inventory. No publication action.
 */

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const npm = process.platform === "win32" ? "npm.cmd" : "npm";

const requireAll = process.argv.includes("--require-all");
const allowPartial = process.argv.includes("--allow-partial");
if (requireAll && allowPartial) {
  console.error("pack-platform: --require-all and --allow-partial are mutually exclusive");
  process.exit(1);
}

const dist = join(root, "ts", "dist", "loader.js");
if (!existsSync(dist)) {
  console.error("pack-platform: ts/dist is missing; run `npm run build` in ts/ first");
  process.exit(1);
}

const { PLATFORM_PACKAGES } = await import(pathToFileURL(dist).href);
const targets = Object.keys(PLATFORM_PACKAGES);
if (targets.length === 0) {
  console.error("pack-platform: the loader exposes no platform packages");
  process.exit(1);
}

const facade = JSON.parse(readFileSync(join(root, "ts", "package.json"), "utf8"));
const version = facade.version;
const release = join(root, "artifacts", "release", version);
const packages = join(release, "packages");
const artifacts = join(root, "artifacts");

const OS = {
  "linux-x64-gnu": "linux",
  "linux-arm64-gnu": "linux",
  "macos-x64": "darwin",
  "macos-arm64": "darwin",
  "windows-x64": "win32",
};
const LIBC = { "linux-x64-gnu": ["glibc"], "linux-arm64-gnu": ["glibc"] };

/** The manifest of one `@sendust/ts-<target>` package. */
function platformManifest(target) {
  const manifest = {
    name: PLATFORM_PACKAGES[target],
    version,
    description: `Sendust native addon for ${target}`,
    os: [OS[target]],
    cpu: [target.split("-")[1]],
    main: "sendust.node",
    files: ["sendust.node"],
    license: facade.license,
    private: false,
  };
  if (LIBC[target]) {
    manifest.libc = LIBC[target];
  }
  return manifest;
}

const sha256 = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");

// A tarball from an earlier partial run is not part of this inventory.
if (existsSync(packages)) {
  for (const name of readdirSync(packages)) {
    if (name.endsWith(".tgz")) {
      rmSync(join(packages, name), { force: true });
    }
  }
} else {
  mkdirSync(packages, { recursive: true });
}

const packed = [];
const missing = [];

for (const target of targets) {
  const addon = join(artifacts, `sendust-${target}.node`);
  if (!existsSync(addon)) {
    missing.push(target);
    continue;
  }

  const manifest = platformManifest(target);
  const staging = mkdtempSync(join(tmpdir(), `sendust-pack-${target}-`));
  try {
    writeFileSync(join(staging, "package.json"), `${JSON.stringify(manifest, null, 2)}\n`);
    copyFileSync(addon, join(staging, "sendust.node"));

    const produced = execFileSync(npm, ["pack", "--silent"], {
      cwd: staging,
      encoding: "utf8",
      shell: process.platform === "win32",
    })
      .trim()
      .split("\n")
      .pop();
    const file = join(packages, produced);
    copyFileSync(join(staging, produced), file);

    packed.push({
      name: manifest.name,
      target,
      file: `packages/${produced}`,
      bytes: statSync(file).size,
      sha256: sha256(file),
      os: manifest.os,
      cpu: manifest.cpu,
      libc: manifest.libc ?? null,
    });
  } finally {
    rmSync(staging, { recursive: true, force: true });
  }
}

if (packed.length === 0) {
  console.error("pack-platform: no artifacts found; run `npm run build:artifact` first");
  process.exit(1);
}
if (missing.length > 0 && requireAll) {
  for (const target of missing) {
    console.error(`pack-platform: missing artifact for ${PLATFORM_PACKAGES[target]} (${target})`);
  }
  process.exit(1);
}
if (missing.length > 0) {
  console.log(`pack-platform: ${missing.length} target(s) without an artifact: ${missing.join(", ")}`);
}

const inventory = {
  schemaVersion: 1,
  package: "@sendust/ts",
  version,
  platformPackages: packed,
  missing,
};
writeFileSync(join(release, "platform-packages.json"), `${JSON.stringify(inventory, null, 2)}\n`);
writeFileSync(
  join(release, "PLATFORM-PACKAGES.sha256"),
  `${packed.map((entry) => `${entry.sha256}  ${entry.name}-${version}.tgz`).join("\n")}\n`,
);

for (const entry of packed) {
  console.log(`pack-platform: ${entry.name}@${version} ${entry.file} ${entry.sha256}`);
}
if (missing.length > 0) {
  process.exit(2);
}
