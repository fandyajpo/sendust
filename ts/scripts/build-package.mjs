#!/usr/bin/env node
/**
 * Builds the installable `@sendust/ts` package.
 *
 * Compiles `src` to ESM JavaScript and declarations under `dist/`, then copies
 * the staged Node-API addon to the package root, where `loader.ts` resolves it
 * (`../sendust.node` from `dist/`). The addon must already be staged by
 * `npm run build:native` at the repository root.
 *
 * Only `dist/` and `sendust.node` are written. Test files are not compiled.
 */

import { copyFileSync, existsSync, mkdirSync, rmSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = join(packageRoot, "..");
const staged = join(packageRoot, "sendust.node");
const stagedSource = join(repoRoot, "ts", "sendust.node");
const dist = join(packageRoot, "dist");

if (!existsSync(stagedSource)) {
  console.error("sendust.node is not staged; run `npm run build:native` first");
  process.exit(1);
}

rmSync(dist, { recursive: true, force: true });

// Flags match the emit verified with TypeScript 7.0.2 (ESM output, .js
// specifiers, declarations). Passing files explicitly needs --ignoreConfig.
execFileSync(
  join(repoRoot, "node_modules", ".bin", "tsc"),
  [
    "src/index.ts",
    "src/types.ts",
    "--ignoreConfig",
    "--target",
    "ES2022",
    "--module",
    "nodenext",
    "--moduleResolution",
    "nodenext",
    "--strict",
    "--verbatimModuleSyntax",
    "--rewriteRelativeImportExtensions",
    "--allowImportingTsExtensions",
    "--declaration",
    "--skipLibCheck",
    "--outDir",
    dist,
  ],
  { cwd: packageRoot, stdio: "inherit" },
);

mkdirSync(packageRoot, { recursive: true });
copyFileSync(stagedSource, staged);
console.log(`built ${dist} and bundled ${staged}`);
