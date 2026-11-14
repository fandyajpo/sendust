#!/usr/bin/env node
/**
 * Static architecture guard for the Sendust repository.
 *
 * Verifies the approved two-crate workspace, the one-way `sendust-napi` ->
 * `sendust-core` dependency, the ADR-0016 module layering, SMTP adapter
 * containment, and runtime-neutral shared TypeScript sources.
 *
 * `cargo metadata` requires the pinned Rust toolchain (rust-toolchain.toml).
 * Nothing in this check starts a JavaScript runtime from the Rust side.
 */

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(fileURLToPath(new URL("..", import.meta.url)));
const CORE_SRC = join(ROOT, "crates/sendust-core/src");
const NAPI_SRC = join(ROOT, "crates/sendust-napi/src");
const TS_SRC = join(ROOT, "ts/src");
const IGNORED_DIRECTORIES = new Set(["target", "node_modules", ".git", "dist"]);

const EXPECTED_MANIFESTS = ["crates/sendust-core/Cargo.toml", "crates/sendust-napi/Cargo.toml"];
const EXPECTED_PACKAGES = ["sendust-core", "sendust-napi"];
const FORBIDDEN_IN_CORE = ["napi", "napi-derive", "napi-build", "napi-sys", "neon", "node-bindgen"];

const MODULE_LAYERS = new Map([
  ["address", "domain"],
  ["attachments", "domain"],
  ["config", "domain"],
  ["errors", "domain"],
  ["message", "domain"],
  ["mime", "domain"],
  ["results", "domain"],
  ["validation", "application"],
  ["delivery", "application"],
  ["lifecycle", "application"],
  ["ports", "port"],
  ["transports", "adapter"],
]);

const ALLOWED_TARGET_LAYERS = new Map([
  ["domain", new Set(["domain"])],
  ["application", new Set(["domain", "application", "port"])],
  ["port", new Set(["domain"])],
  ["adapter", new Set(["domain", "port"])],
]);

const NODE_ONLY_PATTERNS = [
  /\bprocess\s*\./,
  /\bBuffer\b/,
  /\brequire\s*\(/,
  /\b__dirname\b/,
  /\b__filename\b/,
  /from\s+["']node:/,
  /from\s+["'](?:fs|path|os|child_process)["']/,
];

const failures = [];
const fail = (message) => failures.push(message);
const posix = (path) => relative(ROOT, path).split(sep).join("/");

function walk(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      return IGNORED_DIRECTORIES.has(entry.name) ? [] : walk(path);
    }
    return [path];
  });
}

function stripComments(source) {
  return source.replace(/\/\*[\s\S]*?\*\//g, "").replace(/\/\/.*$/gm, "");
}

function checkWorkspaceShape() {
  const manifests = walk(join(ROOT, "crates"))
    .filter((path) => path.endsWith("Cargo.toml"))
    .map(posix)
    .sort();
  if (manifests.join(",") !== EXPECTED_MANIFESTS.join(",")) {
    fail(`expected exactly ${EXPECTED_MANIFESTS.join(", ")}; found ${manifests.join(", ")}`);
  }

  const crateDirectories = readdirSync(join(ROOT, "crates"), { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => `crates/${entry.name}`)
    .sort();
  if (crateDirectories.join(",") !== "crates/sendust-core,crates/sendust-napi") {
    fail(
      `crates/ must contain exactly sendust-core and sendust-napi; found ${crateDirectories.join(", ")}`,
    );
  }

  let metadata;
  try {
    metadata = JSON.parse(
      execFileSync("cargo", ["metadata", "--format-version", "1", "--no-deps"], {
        cwd: ROOT,
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
      }),
    );
  } catch (error) {
    console.error("check-architecture: `cargo metadata` failed; install the pinned toolchain.");
    console.error(String(error.stderr ?? error.message).trim());
    process.exit(1);
  }

  const packages = new Map(metadata.packages.map((pkg) => [pkg.name, pkg]));
  const names = [...packages.keys()].sort();
  if (names.join(",") !== EXPECTED_PACKAGES.join(",")) {
    fail(`workspace packages must be exactly ${EXPECTED_PACKAGES.join(", ")}; found ${names.join(", ")}`);
    return;
  }

  const core = packages.get("sendust-core");
  const napi = packages.get("sendust-napi");
  // Dev- and build-dependencies are filtered out; lettre is forbidden in every
  // kind because Sendust owns the SMTP engine (ADR-0017).
  const productionDependencies = (pkg) =>
    new Set(
      pkg.dependencies
        .filter((dependency) => dependency.kind === null)
        .map((dependency) => dependency.name),
    );
  const coreDependencies = productionDependencies(core);
  const napiDependencies = productionDependencies(napi);

  for (const forbidden of FORBIDDEN_IN_CORE) {
    if (coreDependencies.has(forbidden)) {
      fail(`sendust-core must not depend on ${forbidden}`);
    }
  }
  if (coreDependencies.has("lettre")) {
    fail("sendust-core must not depend on lettre in production (ADR-0017)");
  }
  if (!napiDependencies.has("sendust-core")) {
    fail("sendust-napi must depend on sendust-core");
  }
  if (napiDependencies.has("lettre")) {
    fail("sendust-napi must not depend on lettre directly");
  }
  if (core.targets.some((target) => target.kind.includes("custom-build"))) {
    fail("sendust-core must not define a build script");
  }
  if (!napi.targets.some((target) => target.crate_types.includes("cdylib"))) {
    fail("sendust-napi must expose a cdylib target");
  }
}

function checkCoreModules() {
  const directories = readdirSync(CORE_SRC, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();
  const expected = [...MODULE_LAYERS.keys()].sort();
  if (directories.join(",") !== expected.join(",")) {
    fail(`sendust-core modules must be exactly ${expected.join(", ")}; found ${directories.join(", ")}`);
    return;
  }

  const edges = new Map();
  for (const path of walk(CORE_SRC).filter((file) => file.endsWith(".rs"))) {
    const relativePath = relative(CORE_SRC, path).split(sep).join("/");
    const owner = relativePath === "lib.rs" ? "facade" : relativePath.split("/")[0];
    const code = stripComments(readFileSync(path, "utf8"));

    for (const match of code.matchAll(/crate::([a-z_]+)/g)) {
      const target = match[1];
      if (!MODULE_LAYERS.has(target) || target === owner) continue;
      if (!edges.has(owner)) edges.set(owner, new Set());
      edges.get(owner).add(target);
    }

    if (code.includes("lettre::")) {
      fail(`sendust source must not reference lettre (ADR-0017): ${posix(path)}`);
    }
    if (MODULE_LAYERS.get(owner) === "domain") {
      for (const forbidden of ["crate::transports", "napi::"]) {
        if (code.includes(forbidden)) {
          fail(`domain module ${owner} must not reference ${forbidden}: ${posix(path)}`);
        }
      }
    }
    if ((owner === "delivery" || owner === "ports") && code.includes("crate::transports")) {
      fail(`${owner} must depend on the port contract, not a concrete transport: ${posix(path)}`);
    }
  }

  for (const [owner, targets] of edges) {
    if (owner === "facade") continue;
    const allowed = ALLOWED_TARGET_LAYERS.get(MODULE_LAYERS.get(owner));
    for (const target of targets) {
      if (!allowed.has(MODULE_LAYERS.get(target))) {
        fail(
          `${MODULE_LAYERS.get(owner)} module ${owner} must not depend on ${MODULE_LAYERS.get(target)} module ${target}`,
        );
      }
    }
  }

  const visiting = new Set();
  const visited = new Set();
  const visit = (node, trail) => {
    if (visited.has(node)) return;
    if (visiting.has(node)) {
      fail(`circular module dependency: ${[...trail, node].join(" -> ")}`);
      return;
    }
    visiting.add(node);
    for (const next of edges.get(node) ?? []) visit(next, [...trail, node]);
    visiting.delete(node);
    visited.add(node);
  };
  for (const node of ["facade", ...MODULE_LAYERS.keys()]) visit(node, []);
}

function checkAdapterBoundaries() {
  for (const path of walk(NAPI_SRC).filter((file) => file.endsWith(".rs"))) {
    const code = stripComments(readFileSync(path, "utf8"));
    if (code.includes("lettre::") || code.includes("crate::transports")) {
      fail(`the NAPI adapter must not contain SMTP or transport policy: ${posix(path)}`);
    }
  }

  for (const path of walk(TS_SRC).filter((file) => file.endsWith(".ts"))) {
    if (path.endsWith("loader.ts")) continue;
    const code = stripComments(readFileSync(path, "utf8"));
    const hits = NODE_ONLY_PATTERNS.filter((pattern) => pattern.test(code)).map(String);
    if (hits.length > 0) {
      fail(`shared TypeScript must stay runtime-neutral: ${posix(path)} uses ${hits.join(", ")}`);
    }
  }
}

function checkTestHarness() {
  const testsDir = join(ROOT, "crates/sendust-core/tests");
  const harnessPath = join(testsDir, "support/mod.rs");
  if (!existsSync(harnessPath)) {
    fail("missing fake SMTP harness at crates/sendust-core/tests/support/mod.rs");
    return;
  }

  const harness = stripComments(readFileSync(harnessPath, "utf8"));
  if (!harness.includes('"127.0.0.1"')) {
    fail("the fake SMTP harness must bind the loopback address 127.0.0.1");
  }

  for (const path of walk(testsDir).filter((file) => file.endsWith(".rs"))) {
    const source = readFileSync(path, "utf8");
    if (stripComments(source).includes("0.0.0.0")) {
      fail(`test code must not bind 0.0.0.0: ${posix(path)}`);
    }
    if (/["']\s*https?:\/\//.test(source)) {
      fail(`test code must not use the network: ${posix(path)}`);
    }
  }

  const tlsDir = join(ROOT, "tests/fixtures/tls");
  for (const name of [
    "ca.crt",
    "valid-server.crt",
    "self-signed.crt",
    "expired.crt",
    "wrong-host.crt",
    "incomplete-chain.crt",
    "malformed.pem",
  ]) {
    if (!existsSync(join(tlsDir, name))) {
      fail(`missing TLS fixture tests/fixtures/tls/${name}`);
    }
  }

  // The test CA and intermediate signing keys must never be committed: only
  // certificates and the leaf keys needed to serve them belong in the repo.
  for (const name of ["ca.key", "intermediate.key"]) {
    if (existsSync(join(tlsDir, name))) {
      fail(`TLS fixtures must not contain a signing key: tests/fixtures/tls/${name}`);
    }
  }

  const leafKeys = new Set([
    "valid-server.key",
    "self-signed.key",
    "expired.key",
    "wrong-host.key",
    "incomplete-chain.key",
  ]);
  for (const name of readdirSync(tlsDir)) {
    if (name.endsWith(".key") && !leafKeys.has(name)) {
      fail(`unexpected private key in TLS fixtures: tests/fixtures/tls/${name}`);
    }
  }
}

checkWorkspaceShape();
checkCoreModules();
checkAdapterBoundaries();
checkTestHarness();

if (failures.length > 0) {
  console.error(`check-architecture: ${failures.length} problem(s)`);
  for (const failure of failures) console.error(`  - ${failure}`);
  process.exit(1);
}

console.log(
  "PASS: two crates, one-way napi -> core, ADR-0016 module layering, SMTP adapter containment, runtime-neutral TypeScript",
);
