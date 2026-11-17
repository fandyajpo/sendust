#!/usr/bin/env node
/**
 * Verifies the packed `@sendust/ts` tarball contains exactly the allowlist.
 *
 * Runs `npm pack --dry-run --json` from the package directory and fails if any
 * file falls outside the allowed set, or if a forbidden artifact (local
 * environment files, tests, fixtures, docs, source tree, or the workspace
 * root) is present. The allowlist is the security boundary: `.npmignore` is
 * deliberately not relied on.
 *
 * Requires `npm run build` first, so `dist/` and `sendust.node` exist.
 */

import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

const built =
  existsSync(join(packageRoot, "dist", "index.js")) && existsSync(join(packageRoot, "sendust.node"));
if (!built) {
  console.error("dist/ or sendust.node is missing; run `npm run build` first");
  process.exit(1);
}

const ALLOWED_PREFIXES = ["dist/"];
const ALLOWED_FILES = new Set(["package.json", "sendust.node", "README.md"]);
const FORBIDDEN = [
  /(^|\/)\.env(\.|$)/,
  /(^|\/)node_modules\//,
  /^test\//,
  /^scripts\//,
  /^src\//,
  /^tests\//,
  /^docs\//,
  /\.map$/,
  /\.key$/,
  /\.pem$/,
];

const raw = execFileSync("npm", ["pack", "--dry-run", "--json", "--ignore-scripts"], {
  cwd: packageRoot,
  encoding: "utf8",
  stdio: ["ignore", "pipe", "inherit"],
});
const [pack] = JSON.parse(raw);
const paths = pack.files.map((file) => file.path);

const failures = [];
for (const path of paths) {
  const allowed =
    ALLOWED_FILES.has(path) || ALLOWED_PREFIXES.some((prefix) => path.startsWith(prefix));
  if (!allowed) {
    failures.push(`not in the allowlist: ${path}`);
  }
  if (FORBIDDEN.some((pattern) => pattern.test(path))) {
    failures.push(`forbidden artifact: ${path}`);
  }
}

for (const required of ["package.json", "sendust.node", "README.md"]) {
  if (!paths.includes(required)) {
    failures.push(`required file missing from the tarball: ${required}`);
  }
}
if (!paths.some((path) => path === "dist/index.js")) {
  failures.push("required file missing from the tarball: dist/index.js");
}

if (failures.length > 0) {
  for (const failure of failures) {
    console.error(`FAIL: ${failure}`);
  }
  process.exit(1);
}

console.log(`PASS: ${paths.length} files, all within the allowlist (${pack.filename})`);
