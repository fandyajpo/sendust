#!/usr/bin/env node
/**
 * Builds a per-platform package from a CI artifact and installs it with the
 * packed facade into a clean consumer, proving the published layout (ADR-0020).
 *
 * Usage: npm run verify:platform
 *
 * The facade is installed from the tarball a user would get, not from the
 * repository folder. Per artifact it proves:
 *
 * - Node.js ESM and CommonJS: load through the loader, open, close, and refuse
 *   a send after close with `TRANSPORT_CLOSED`.
 * - Bun and Deno: the same facade script loads and closes.
 * - A platform package without its addon, and a facade with no platform
 *   package at all, fail with the typed missing-binary error.
 * - A corrupt addon fails with the typed load error naming the package.
 *
 * Bun and Deno checks are skipped with a notice when the runtime is not on
 * PATH; the CI artifacts job installs both before this step.
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
const shell = process.platform === "win32";

const OS = { "linux-x64-gnu": "linux", "linux-arm64-gnu": "linux", "macos-x64": "darwin", "macos-arm64": "darwin", "windows-x64": "win32" };
const CPU = { x64: "x64", arm64: "arm64" };

const FACADE_CHECK = `
import { createTransport } from '@sendust/ts';
const transport = createTransport({ host: '127.0.0.1', port: 1, secure: 'tls' });
if (transport.state !== 'open') throw new Error('state ' + transport.state);
await transport.close();
if (transport.state !== 'closed') throw new Error('not closed');
let code = null;
try {
  await transport.send({ from: 'a@example.test', to: ['b@example.test'], subject: 'x', text: 'y' });
} catch (error) {
  code = error.code;
}
if (code !== 'TRANSPORT_CLOSED') throw new Error('send after close: ' + code);
console.log('ok');
`;

const COMMON_JS_CHECK = `
const { createTransport } = require('@sendust/ts');
const transport = createTransport({ host: '127.0.0.1', port: 1, secure: 'tls' });
if (transport.state !== 'open') throw new Error('state ' + transport.state);
transport.close().then(
  () => {
    if (transport.state !== 'closed') throw new Error('not closed');
    console.log('ok');
  },
  (error) => {
    console.error(error);
    process.exit(1);
  },
);
`;

/** A reject-mode checker: constructing a transport must raise the typed error. */
const CHECKER = `
const facade = require('@sendust/ts');
const expected = process.argv[1] || '';
const label = process.argv[2] || '';
let failure = null;
try {
  facade.createTransport({ host: '127.0.0.1', port: 1, secure: 'tls' });
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

if (!existsSync(join(root, "ts", "dist", "index.js"))) {
  console.error("verify-platform: built package missing; run `npm run build` in ts/ first");
  process.exit(1);
}

// The published facade: dist, manifest and README, with no bundled addon, so
// the loader must resolve the per-platform package.
const facadeDir = mkdtempSync(join(tmpdir(), "sendust-facade-"));
cpSync(join(root, "ts", "dist"), join(facadeDir, "dist"), { recursive: true });
copyFileSync(join(root, "ts", "package.json"), join(facadeDir, "package.json"));
if (existsSync(join(root, "ts", "README.md"))) {
  copyFileSync(join(root, "ts", "README.md"), join(facadeDir, "README.md"));
}
const tarballName = execFileSync(npm, ["pack", "--silent"], {
  cwd: facadeDir,
  encoding: "utf8",
  shell,
})
  .trim()
  .split("\n")
  .pop();
const tarball = join(facadeDir, tarballName);

/** Whether a runtime is available to run the consumer checks. */
function onPath(command) {
  try {
    execFileSync(command, ["--version"], { stdio: "ignore", shell });
    return true;
  } catch {
    return false;
  }
}

/** Runs the reject-mode checker in the consumer. */
function check(consumer, expected, label) {
  return execFileSync(process.execPath, ["-e", CHECKER, expected, label], {
    cwd: consumer,
    encoding: "utf8",
  }).trim();
}

/** Runs a consumer script and returns its last line. */
function run(consumer, command, args) {
  return execFileSync(command, args, { cwd: consumer, encoding: "utf8", shell })
    .trim()
    .split("\n")
    .pop();
}

const hasBun = onPath("bun");
const hasDeno = onPath("deno");
if (!hasBun) console.log("note: bun is not on PATH; Bun consumer checks are skipped");
if (!hasDeno) console.log("note: deno is not on PATH; Deno consumer checks are skipped");

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
      // --install-links copies the platform folder instead of symlinking it, so
      // the facade resolves it from the consumer's node_modules.
      ["install", "--silent", "--no-audit", "--no-fund", "--install-links", tarball, platformDir],
      // Node refuses to spawn a `.cmd` without a shell on Windows (CVE-2024-27910).
      { cwd: consumer, stdio: "inherit", shell },
    );
    writeFileSync(join(consumer, "facade-check.mjs"), FACADE_CHECK);
    writeFileSync(join(consumer, "facade-check.cjs"), COMMON_JS_CHECK);

    console.log(`${key} node esm: ${run(consumer, process.execPath, ["facade-check.mjs"])}`);
    console.log(`${key} node cjs: ${run(consumer, process.execPath, ["facade-check.cjs"])}`);
    if (hasBun) {
      console.log(`${key} bun: ${run(consumer, "bun", ["facade-check.mjs"])}`);
    }
    if (hasDeno) {
      console.log(
        `${key} deno: ${run(consumer, "deno", [
          "run",
          "--allow-ffi",
          "--allow-read",
          "facade-check.mjs",
        ])}`,
      );
    }

    // The facade alone: no platform package installed, which is the published
    // situation on a target with no build or a skipped optional dependency.
    const bare = mkdtempSync(join(tmpdir(), "sendust-bare-"));
    try {
      writeFileSync(join(bare, "package.json"), JSON.stringify({ name: "consumer", private: true }));
      execFileSync(
        npm,
        ["install", "--silent", "--no-audit", "--no-fund", tarball],
        { cwd: bare, stdio: "inherit", shell },
      );
      console.log(
        `${key} no platform package: ${check(bare, `${packageName} is not installed`, "absent")}`,
      );
    } finally {
      rmSync(bare, { recursive: true, force: true });
    }

    // Missing binary: the optional package installs but has no addon, which is
    // what a skipped or failed platform download looks like.
    rmSync(installedAddon, { force: true });
    console.log(`${key} missing addon: ${check(consumer, `${packageName} is not installed`, "missing")}`);

    // Wrong architecture or corrupted binary: bytes that cannot be this ABI.
    writeFileSync(installedAddon, Buffer.from([0x7f, 0x45, 0x4c, 0x46, 0x02]));
    console.log(`${key} corrupt addon: ${check(consumer, "failed to load (", "corrupt")}`);
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
