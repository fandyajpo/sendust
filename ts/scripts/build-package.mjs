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

import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
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

// CommonJS twin, so `require()` resolves the same surface (C-22, ADR-0012). The
// shared source has no `import.meta`, so the same files compile for both; only
// the module-context placeholder differs.
execFileSync(
  join(repoRoot, "node_modules", ".bin", "tsc"),
  [
    "src/index.ts",
    "src/types.ts",
    "--ignoreConfig",
    "--target",
    "ES2022",
    "--module",
    "commonjs",
    "--strict",
    "--rewriteRelativeImportExtensions",
    "--allowImportingTsExtensions",
    "--declaration",
    "--skipLibCheck",
    "--outDir",
    join(dist, "cjs"),
  ],
  { cwd: packageRoot, stdio: "inherit" },
);

// Swap the module URL placeholder: `import.meta.url` is only legal in the ESM
// output, and `__filename` only in the CommonJS output.
rewriteModuleUrl(join(dist, "module-context.js"), "import.meta.url");
rewriteModuleUrl(
  join(dist, "cjs", "module-context.js"),
  'require("node:url").pathToFileURL(__filename).href',
);

// The package is `"type": "module"`, so CommonJS output must use `.cjs` or Node
// would treat it as ESM. Rename and fix the relative require specifiers.
for (const file of listFiles(join(dist, "cjs"))) {
  if (!file.endsWith(".js")) continue;
  const source = readFileSync(file, "utf8")
    .replace(/require\("(\.[^"]+)\.js"\)/g, 'require("$1.cjs")')
    // The CommonJS output sits one level deeper, so the addon at the package
    // root is one more directory up.
    .replace('"../sendust.node"', '"../../sendust.node"');
  const target = file.replace(/\.js$/, ".cjs");
  writeFileSync(target, source);
  rmSync(file);
}

mkdirSync(packageRoot, { recursive: true });
copyFileSync(stagedSource, staged);
console.log(`built ${dist} (ESM and CommonJS) and bundled ${staged}`);

/** Replaces the placeholder exactly once and fails loudly otherwise. */
function rewriteModuleUrl(file, replacement) {
  const source = readFileSync(file, "utf8");
  const marker = '"SENDUST_MODULE_URL_PLACEHOLDER"';
  const occurrences = source.split(marker).length - 1;
  if (occurrences !== 1) {
    console.error(`${file}: expected one module URL placeholder, found ${occurrences}`);
    process.exit(1);
  }
  writeFileSync(file, source.replace(marker, replacement));
}

/** Every file below `directory`, recursively. */
function listFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    return entry.isDirectory() ? listFiles(path) : [path];
  });
}
