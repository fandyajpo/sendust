#!/usr/bin/env node
/**
 * Proves the published install layout: the packed facade plus the platform
 * package npm selects for this OS and CPU (ADR-0020).
 *
 * Usage: npm run verify:platform
 *
 * Two facade tarballs are packed from the built package, both without a bundled
 * addon:
 *
 * - "selection": `optionalDependencies` point at the local platform package
 *   folders. That is a test substitution for the published version ranges: the
 *   committed manifest pins `0.1.0`, which only a registry can resolve.
 * - "bare": the same manifest with no `optionalDependencies`, which is the state
 *   after npm skips a platform package.
 *
 * Per artifact the selection consumer proves that npm installed the matching
 * `@sendust/ts-<key>` and none of the other four packages, that the facade loads
 * and closes under Node.js (ESM and CommonJS), Bun, and Deno, and that a missing
 * or corrupt addon fails with the typed error. The bare consumer proves the
 * facade alone installs and fails closed.
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
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const npm = process.platform === "win32" ? "npm.cmd" : "npm";
const shell = process.platform === "win32";

const OS = { "linux-x64-gnu": "linux", "linux-arm64-gnu": "linux", "linux-x64-musl": "linux", "linux-arm64-musl": "linux", "macos-x64": "darwin", "macos-arm64": "darwin", "windows-x64": "win32" };
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

// The loader's map is the single source of truth for the platform packages, so
// this check cannot drift from what the package can actually resolve.
const { PLATFORM_PACKAGES } = await import(
  pathToFileURL(join(root, "ts", "dist", "loader.js")).href
);
const PLATFORM_KEYS = Object.keys(PLATFORM_PACKAGES);

const baseManifest = JSON.parse(readFileSync(join(root, "ts", "package.json"), "utf8"));

/** The manifest of one `@sendust/ts-<platform>-<arch>` package. */
function platformManifest(key) {
  return {
    name: `@sendust/ts-${key}`,
    version: "0.1.0",
    description: `Sendust native addon for ${key}`,
    os: [OS[key]],
    cpu: [CPU[key.split("-")[1]]],
    main: "sendust.node",
    files: ["sendust.node"],
    license: "MIT OR Apache-2.0",
    private: true,
  };
}

/** A platform package folder: the real addon for this target, placeholders elsewhere. */
function buildPlatformDir(key, addonPath) {
  const dir = mkdtempSync(join(tmpdir(), `sendust-${key}-`));
  writeFileSync(join(dir, "package.json"), JSON.stringify(platformManifest(key), null, 2));
  if (addonPath) {
    copyFileSync(addonPath, join(dir, "sendust.node"));
  } else {
    // Never loaded: npm skips this package because its os/cpu do not match.
    writeFileSync(join(dir, "sendust.node"), Buffer.from([0x7f, 0x45, 0x4c, 0x46, 0x02]));
  }
  return dir;
}

/** Packs the facade with `patch` applied to its manifest, without a bundled addon. */
function packFacade(label, patch) {
  const dir = mkdtempSync(join(tmpdir(), `sendust-facade-${label}-`));
  cpSync(join(root, "ts", "dist"), join(dir, "dist"), { recursive: true });
  if (existsSync(join(root, "ts", "README.md"))) {
    copyFileSync(join(root, "ts", "README.md"), join(dir, "README.md"));
  }
  const manifest = structuredClone(baseManifest);
  patch(manifest);
  writeFileSync(join(dir, "package.json"), JSON.stringify(manifest, null, 2));
  const packed = execFileSync(npm, ["pack", "--silent"], { cwd: dir, encoding: "utf8", shell })
    .trim()
    .split("\n")
    .pop();
  return { dir, tarball: join(dir, packed) };
}

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

function install(consumer, tarball) {
  execFileSync(
    npm,
    // --install-links copies the platform folders instead of symlinking them, so
    // deleting or corrupting an installed addon leaves the source intact.
    ["install", "--silent", "--no-audit", "--no-fund", "--install-links", tarball],
    // Node refuses to spawn a `.cmd` without a shell on Windows (CVE-2024-27910).
    { cwd: consumer, stdio: "inherit", shell },
  );
}

const hasBun = onPath("bun");
const hasDeno = onPath("deno");
if (!hasBun) console.log("note: bun is not on PATH; Bun consumer checks are skipped");
if (!hasDeno) console.log("note: deno is not on PATH; Deno consumer checks are skipped");

let failures = 0;
for (const file of files) {
  const key = file.replace(/^sendust-/, "").replace(/\.node$/, "");
  if (!OS[key] || !CPU[key.split("-")[1]]) {
    console.log(`skip ${key}: no platform metadata`);
    continue;
  }
  const packageName = `@sendust/ts-${key}`;
  const dirs = new Map();
  let selection;
  let bare;
  let consumer;
  let bareConsumer;
  try {
    for (const other of PLATFORM_KEYS) {
      dirs.set(other, buildPlatformDir(other, other === key ? join(artifacts, file) : undefined));
    }

    selection = packFacade("selection", (manifest) => {
      manifest.optionalDependencies = Object.fromEntries(
        PLATFORM_KEYS.map((other) => [
          `@sendust/ts-${other}`,
          `file:${dirs.get(other).replace(/\\/g, "/")}`,
        ]),
      );
    });
    bare = packFacade("bare", (manifest) => {
      delete manifest.optionalDependencies;
    });

    consumer = mkdtempSync(join(tmpdir(), "sendust-consumer-"));
    writeFileSync(join(consumer, "package.json"), JSON.stringify({ name: "consumer", private: true }));
    install(consumer, selection.tarball);

    // npm, not this script, must choose the platform package.
    const installed = PLATFORM_KEYS.filter((other) =>
      existsSync(join(consumer, "node_modules", "@sendust", `ts-${other}`)),
    );
    if (installed.length !== 1 || installed[0] !== key) {
      throw new Error(`expected only ${key} installed, found ${installed.join(",") || "none"}`);
    }
    console.log(`${key} selection: installed only ${packageName}`);

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

    // Missing binary: the installed package loses its addon.
    const installedAddon = join(consumer, "node_modules", "@sendust", `ts-${key}`, "sendust.node");
    rmSync(installedAddon, { force: true });
    console.log(`${key} missing addon: ${check(consumer, `${packageName} is not installed`, "missing")}`);

    // Wrong architecture or corrupted binary: bytes that cannot be this ABI.
    writeFileSync(installedAddon, Buffer.from([0x7f, 0x45, 0x4c, 0x46, 0x02]));
    console.log(`${key} corrupt addon: ${check(consumer, "failed to load (", "corrupt")}`);

    // No platform package at all: the facade installs and fails closed.
    bareConsumer = mkdtempSync(join(tmpdir(), "sendust-bare-"));
    writeFileSync(
      join(bareConsumer, "package.json"),
      JSON.stringify({ name: "consumer", private: true }),
    );
    install(bareConsumer, bare.tarball);
    console.log(
      `${key} no platform package: ${check(bareConsumer, `${packageName} is not installed`, "absent")}`,
    );
  } catch (error) {
    failures += 1;
    console.error(`${key}: FAILED (${error instanceof Error ? error.message : String(error)})`);
  } finally {
    if (consumer) rmSync(consumer, { recursive: true, force: true });
    if (bareConsumer) rmSync(bareConsumer, { recursive: true, force: true });
    if (selection) rmSync(selection.dir, { recursive: true, force: true });
    if (bare) rmSync(bare.dir, { recursive: true, force: true });
    for (const dir of dirs.values()) rmSync(dir, { recursive: true, force: true });
  }
}

if (failures > 0) {
  console.error(`verify-platform: ${failures} platform(s) failed`);
  process.exit(1);
}
console.log("verify-platform: npm selected the matching platform package on every artifact");
