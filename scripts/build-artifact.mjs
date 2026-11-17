#!/usr/bin/env node
/**
 * Builds and names the native addon for the host platform (ADR-0020).
 *
 * Runs on a native CI runner: `cargo build -p sendust-napi --release` for the
 * runner's own target, then copies the cdylib to
 * `artifacts/sendust-<platform>-<arch>[-<libc>].node`.
 *
 * No cross-compilation: a target is only produced by a runner that can load it.
 */

import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

/** `linux-x64-gnu`, `macos-arm64`, `windows-x64`, ... */
function platformKey() {
  const { platform, arch } = process;
  if (platform === "linux") {
    const libc = process.report?.getReport?.().header?.glibcVersionRuntime ? "gnu" : "musl";
    return `linux-${arch}-${libc}`;
  }
  if (platform === "darwin") {
    return `macos-${arch}`;
  }
  if (platform === "win32") {
    return `windows-${arch}`;
  }
  throw new Error(`no artifact name for ${platform}-${arch}`);
}

const key = platformKey();
const artifact = `sendust-${key}.node`;

execFileSync("cargo", ["build", "-p", "sendust-napi", "--release", "--locked"], {
  cwd: root,
  stdio: "inherit",
});

const library =
  process.platform === "win32"
    ? "sendust_napi.dll"
    : process.platform === "darwin"
      ? "libsendust_napi.dylib"
      : "libsendust_napi.so";
const built = join(root, "target", "release", library);
if (!existsSync(built)) {
  console.error(`build-artifact: ${built} was not produced`);
  process.exit(1);
}

const artifacts = join(root, "artifacts");
mkdirSync(artifacts, { recursive: true });
const out = join(artifacts, artifact);
copyFileSync(built, out);
console.log(`build-artifact: staged ${out}`);
