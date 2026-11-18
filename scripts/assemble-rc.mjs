#!/usr/bin/env node
/**
 * Assembles the release-candidate evidence for the current version: artifact
 * checksums, a dependency inventory, and provenance metadata. It never
 * publishes anything.
 *
 * Usage: node scripts/assemble-rc.mjs
 * Run after `npm run build:artifact` (or `npm run verify`).
 *
 * Output: artifacts/release/<version>/{provenance.json,dependencies.json,SHA256SUMS}
 *
 * The script fails on an artifact whose platform key is not in the supported
 * set, on a missing artifact, or on a secret-scan failure. An empty artifact
 * list is allowed only with `--allow-empty` (used by unit-style runs); the RC
 * candidate requires at least one.
 */

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (path) => readFileSync(join(ROOT, path), "utf8");
const json = (path) => JSON.parse(read(path));

const SUPPORTED = {
  "linux-x64-gnu": ["linux", "x64"],
  "linux-arm64-gnu": ["linux", "arm64"],
  "macos-x64": ["darwin", "x64"],
  "macos-arm64": ["darwin", "arm64"],
  "windows-x64": ["win32", "x64"],
};

const allowEmpty = process.argv.includes("--allow-empty");
const failures = [];
const fail = (message) => failures.push(message);

/** Runs a command and returns its trimmed stdout, or "unavailable". */
function tool(command, args = ["--version"]) {
  try {
    return execFileSync(command, args, { cwd: ROOT, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
  } catch {
    return "unavailable";
  }
}

// Secrets must not reach an artifact or the evidence.
try {
  execFileSync(process.execPath, ["scripts/check-secrets.mjs"], { cwd: ROOT, stdio: "ignore" });
} catch {
  fail("secret scan failed; refusing to assemble release evidence");
}

const version = json("package.json").version;
const commit = tool("git", ["rev-parse", "HEAD"]);
const dirty = tool("git", ["status", "--porcelain"]) !== "";

// Artifacts present in artifacts/, validated against the supported matrix.
const artifactsDir = join(ROOT, "artifacts");
const artifacts = existsSync(artifactsDir)
  ? readdirSync(artifactsDir)
      .filter((name) => name.startsWith("sendust-") && name.endsWith(".node"))
      .sort()
      .map((name) => {
        const path = join(artifactsDir, name);
        const key = name.replace(/^sendust-/, "").replace(/\.node$/, "");
        if (!SUPPORTED[key]) {
          fail(`${name}: not a supported target; refusing to publish it`);
        }
        return {
          name,
          target: key,
          bytes: statSync(path).size,
          sha256: createHash("sha256").update(readFileSync(path)).digest("hex"),
        };
      })
  : [];

if (artifacts.length === 0 && !allowEmpty) {
  fail("no artifacts found; run `npm run build:artifact` first");
}

// Dependency inventory: Rust crates from the lockfile, npm packages from the
// root lockfile. This is a plain inventory, not a full SBOM format.
const cargoLock = read("Cargo.lock");
const rust = [...cargoLock.matchAll(/\[\[package\]\]\nname = "([^"]+)"\nversion = "([^"]+)"/g)].map(
  ([, name, version]) => ({ name, version }),
);
const nodeLock = json("package-lock.json");
const npm = Object.entries(nodeLock.packages ?? {})
  .filter(([path]) => path !== "")
  .map(([path, entry]) => ({ name: path.replace(/^.*node_modules\//, ""), version: entry.version ?? "unknown" }));

const dependencies = { rust, npm };

const typescript = json("ts/package.json").devDependencies ?? {};
const provenance = {
  schemaVersion: 1,
  package: "@sendust/ts",
  version,
  commit,
  dirty,
  note: "Assembled by scripts/assemble-rc.mjs. No publication action has been taken.",
  toolchain: {
    rustc: tool("rustc"),
    cargo: tool("cargo"),
    node: tool("node"),
    npm: tool("npm"),
    typescript: typescript.typescript ?? "unknown",
    biome: typescript["@biomejs/biome"] ?? "unknown",
    vitest: typescript.vitest ?? "unknown",
    bun: tool("bun"),
    deno: tool("deno"),
  },
  artifacts,
};

if (failures.length > 0) {
  for (const message of failures) {
    console.error(`FAIL: ${message}`);
  }
  process.exit(1);
}

const outDir = join(artifactsDir, "release", version);
rmSync(outDir, { recursive: true, force: true });
mkdirSync(outDir, { recursive: true });
writeFileSync(join(outDir, "provenance.json"), `${JSON.stringify(provenance, null, 2)}\n`);
writeFileSync(join(outDir, "dependencies.json"), `${JSON.stringify(dependencies, null, 2)}\n`);
writeFileSync(
  join(outDir, "SHA256SUMS"),
  `${artifacts.map((artifact) => `${artifact.sha256}  ${artifact.name}`).join("\n")}${artifacts.length ? "\n" : ""}`,
);

console.log(`release evidence: ${outDir}`);
console.log(`  version ${version} at ${commit}${dirty ? " (working tree dirty)" : ""}`);
console.log(`  ${artifacts.length} artifact(s), ${rust.length} Rust crates, ${npm.length} npm packages`);
