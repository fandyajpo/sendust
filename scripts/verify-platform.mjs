#!/usr/bin/env node
/**
 * Builds a per-platform package from a CI artifact and installs it with the
 * facade into a clean consumer, proving the loader resolves it (ADR-0020).
 *
 * Usage: npm run verify:platform
 *
 * Uses every `artifacts/sendust-*.node` present. The facade tarball is built
 * first, so run `npm run build` in `ts/` (or `npm run verify`) beforehand.
 *
 * Per artifact it checks three load paths in the shipped layout: the addon
 * loads and the transport opens and closes, a package without its addon fails
 * with the typed missing-binary error, and a corrupt addon fails with the typed
 * load error naming the package (P14-004 negatives).
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

/**
 * One child process per scenario, so each loads the facade into a fresh process
 * after the consumer's `node_modules` changed underneath it.
 */
const CHECKER = `
const facade = require('@sendust/ts');
const mode = process.argv[1];
const expected = process.argv[2] || '';
const label = process.argv[3] || '';
function construct() {
  return facade.createTransport({ host: '127.0.0.1', port: 1, secure: 'tls' });
}
if (mode === 'ok') {
  const transport = construct();
  if (transport.state !== 'open') throw new Error('state ' + transport.state);
  transport.close().then(() => {
    if (transport.state !== 'closed') throw new Error('not closed');
    console.log('ok: open then closed');
  });
} else {
  let failure = null;
  try {
    construct();
  } catch (caught) {
    failure = caught;
  }
  if (!failure) throw new Error('expected the native load to fail');
  if (failure.code !== 'CONFIG_NATIVE_UNAVAILABLE') {
    throw new Error('unexpected code ' + failure.code);
  }
  if (!failure.message.includes(expected)) {
    throw new Error('message lacks "' + expected + '": ' + failure.message);
  }
  console.log('typed ' + failure.code + ' (' + label + ')');
}
`;

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

/** Runs the checker in the consumer and returns its one-line report. */
function check(consumer, args) {
  return execFileSync(process.execPath, ["-e", CHECKER, ...args], {
    cwd: consumer,
    encoding: "utf8",
  }).trim();
}

let failures = 0;
for (const file of files) {
  const key = file.replace(/^sendust-/, "").replace(/\.node$/, "");
  const os = OS[key];
  const arch = key.split("-")[1];
  if (!os || !CPU[arch]) {
    console.log(`skip ${key}: no platform metadata`);
    continue;
  }
  const packageName = `@sendust/ts-${key}`;

  const platformDir = mkdtempSync(join(tmpdir(), `sendust-${key}-`));
  writeFileSync(
    join(platformDir, "package.json"),
    JSON.stringify(
      {
        name: packageName,
        version: "0.1.0",
        description: `Sendust native addon for ${key}`,
        os: [os],
        cpu: [arch],
        main: "sendust.node",
        files: ["sendust.node"],
        license: "MIT OR Apache-2.0",
        private: true,
      },
      null,
      2,
    ),
  );
  copyFileSync(join(artifacts, file), join(platformDir, "sendust.node"));

  const consumer = mkdtempSync(join(tmpdir(), "sendust-consumer-"));
  const installedAddon = join(consumer, "node_modules", "@sendust", `ts-${key}`, "sendust.node");
  try {
    writeFileSync(join(consumer, "package.json"), JSON.stringify({ name: "consumer", private: true }));
    execFileSync(
      npm,
      // --install-links copies the folders instead of symlinking them, so the
      // facade resolves the platform package from the consumer's node_modules.
      ["install", "--silent", "--no-audit", "--no-fund", "--install-links", facadeDir, platformDir],
      { cwd: consumer, stdio: "inherit" },
    );

    console.log(`${key} load: ${check(consumer, ["ok", "", "load"])}`);

    // Missing binary: the optional package installs but has no addon, which is
    // what a skipped or failed platform download looks like.
    rmSync(installedAddon, { force: true });
    console.log(`${key} missing: ${check(consumer, ["reject", `${packageName} is not installed`, "missing"])}`);

    // Wrong architecture or corrupted binary: bytes that cannot be this ABI.
    writeFileSync(installedAddon, Buffer.from([0x7f, 0x45, 0x4c, 0x46, 0x02]));
    console.log(`${key} corrupt: ${check(consumer, ["reject", "failed to load (", "corrupt"])}`);
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
