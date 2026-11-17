#!/usr/bin/env node
/**
 * Builds a per-platform package from a CI artifact and installs it with the
 * facade into a clean consumer, proving the loader resolves it (ADR-0020).
 *
 * Usage: npm run verify:platform
 *
 * Uses every `artifacts/sendust-*.node` present. The facade tarball is built
 * first, so run `npm run build` in `ts/` (or `npm run verify`) beforehand.
 */

import { execFileSync } from "node:child_process";
import {
  copyFileSync,
  cpSync,
  existsSync,
  mkdtempSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const npm = process.platform === "win32" ? "npm.cmd" : "npm";

const OS = { "linux-x64-gnu": "linux", "linux-arm64-gnu": "linux", "macos-x64": "darwin", "macos-arm64": "darwin", "windows-x64": "win32" };
const CPU = { x64: "x64", arm64: "arm64" };

const artifacts = join(root, "artifacts");
if (!existsSync(artifacts)) {
  console.error("verify-platform: no artifacts/ directory; run `npm run build:artifact` first");
  process.exit(1);
}

const files = readdirSync(artifacts).filter((name) => name.startsWith("sendust-") && name.endsWith(".node"));
if (files.length === 0) {
  console.error("verify-platform: no artifacts found");
  process.exit(1);
}

// Facade package without a bundled addon, so the loader must resolve the
// per-platform package instead of the local build.
const facadeDir = mkdtempSync(join(tmpdir(), "sendust-facade-"));
if (!existsSync(join(root, "ts", "dist", "index.js"))) {
  console.error("verify-platform: built package missing; run `npm run build` in ts/ first");
  process.exit(1);
}
cpSync(join(root, "ts", "dist"), join(facadeDir, "dist"), { recursive: true });
copyFileSync(join(root, "ts", "package.json"), join(facadeDir, "package.json"));

let failures = 0;
for (const file of files) {
  const key = file.replace(/^sendust-/, "").replace(/\.node$/, "");
  const os = OS[key];
  const arch = key.split("-")[1];
  if (!os || !CPU[arch]) {
    console.log(`skip ${key}: no platform metadata`);
    continue;
  }

  const platformDir = mkdtempSync(join(tmpdir(), `sendust-${key}-`));
  writeFileSync(
    join(platformDir, "package.json"),
    JSON.stringify(
      {
        name: `@sendust/ts-${key}`,
        version: "0.1.0",
        description: `Sendust native addon for ${key}`,
        os: [os],
        cpu: [arch],
        main: "sendust.node",
        files: ["sendust.node"],
        license: "UNLICENSED",
        private: true,
      },
      null,
      2,
    ),
  );
  copyFileSync(join(artifacts, file), join(platformDir, "sendust.node"));

  const consumer = mkdtempSync(join(tmpdir(), "sendust-consumer-"));
  try {
    writeFileSync(join(consumer, "package.json"), JSON.stringify({ name: "consumer", private: true }));
    execFileSync(npm, ["install", "--silent", "--no-audit", "--no-fund", facadeDir, platformDir], {
      cwd: consumer,
      stdio: "inherit",
    });
    const script = [
      "const s = require('@sendust/ts');",
      "const t = s.createTransport({ host: '127.0.0.1', port: 1, secure: 'tls' });",
      "if (t.state !== 'open') throw new Error('state ' + t.state);",
      "t.close().then(() => { if (t.state !== 'closed') throw new Error('not closed'); console.log('ok'); });",
    ].join(" ");
    const out = execFileSync(process.execPath, ["-e", script], { cwd: consumer, encoding: "utf8" });
    console.log(`${key}: ${out.trim()}`);
  } catch (error) {
    failures += 1;
    console.error(`${key}: FAILED (${error instanceof Error ? error.message : String(error)})`);
  } finally {
    rmSync(consumer, { recursive: true, force: true });
    rmSync(platformDir, { recursive: true, force: true });
  }
}

rmSync(facadeDir, { recursive: true, force: true });
if (failures > 0) {
  console.error(`verify-platform: ${failures} platform(s) failed`);
  process.exit(1);
}
console.log("verify-platform: all artifacts resolved through @sendust/ts");
