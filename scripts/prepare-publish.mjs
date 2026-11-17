#!/usr/bin/env node
/**
 * Stages and packs the publishable facade with its optional platform packages.
 *
 * `optionalDependencies` cannot live in `ts/package.json`: npm requires a
 * lockfile entry for every declared dependency, and the platform packages do not
 * exist until they are published, so `npm ci` fails with "Missing: … from lock
 * file". The published tarball must still declare them, so this script builds
 * that manifest from the loader's own map, pins each package to the facade
 * version, and packs the result (ADR-0020).
 *
 * Usage: npm run prepare:publish
 * Run after `npm run build --workspace ts` (or `npm run verify`).
 */

import { execFileSync } from "node:child_process";
import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = join(fileURLToPath(import.meta.url), "..", "..");
const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const dist = join(root, "ts", "dist");
const staging = join(root, "artifacts", "publish");

if (!existsSync(join(dist, "index.js"))) {
  console.error("prepare-publish: ts/dist is missing; run `npm run build --workspace ts` first");
  process.exit(1);
}

const { PLATFORM_PACKAGES } = await import(pathToFileURL(join(dist, "loader.js")).href);
const platformPackages = Object.values(PLATFORM_PACKAGES);
if (platformPackages.length === 0) {
  console.error("prepare-publish: the loader exposes no platform packages");
  process.exit(1);
}

const manifest = JSON.parse(await readFile(join(root, "ts", "package.json"), "utf8"));
manifest.optionalDependencies = Object.fromEntries(
  platformPackages.map((name) => [name, manifest.version]),
);

await rm(staging, { recursive: true, force: true });
await mkdir(staging, { recursive: true });
await cp(dist, join(staging, "dist"), { recursive: true });
if (existsSync(join(root, "ts", "README.md"))) {
  await cp(join(root, "ts", "README.md"), join(staging, "README.md"));
}
await writeFile(join(staging, "package.json"), JSON.stringify(manifest, null, 2));

const packed = execFileSync(npm, ["pack", "--silent"], {
  cwd: staging,
  encoding: "utf8",
  shell: process.platform === "win32",
})
  .trim()
  .split("\n")
  .pop();
const tarball = join(staging, packed);

// The publish manifest must declare exactly the loader's packages, at the
// facade version; anything else would ship a facade that cannot resolve an addon.
const declared = Object.keys(manifest.optionalDependencies);
if (declared.length !== platformPackages.length) {
  console.error(
    `prepare-publish: ${declared.length} optional dependencies for ${platformPackages.length} platforms`,
  );
  process.exit(1);
}
for (const [name, version] of Object.entries(manifest.optionalDependencies)) {
  if (version !== manifest.version) {
    console.error(`prepare-publish: ${name} is pinned to ${version}, not ${manifest.version}`);
    process.exit(1);
  }
}

console.log(`prepare-publish: ${tarball}`);
console.log(`prepare-publish: optional dependencies ${declared.join(", ")}`);
