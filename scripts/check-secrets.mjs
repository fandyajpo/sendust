#!/usr/bin/env node
/**
 * Deterministic secret scan for the Sendust repository.
 *
 * Scans every tracked text file for high-signal credential shapes. The TLS test
 * fixtures are the single explicit exception: their leaf private keys exist
 * only to serve loopback certificates in tests (see
 * docs/engineering/test-harness-contract.md) and authenticate nothing.
 *
 * Usage: node scripts/check-secrets.mjs [--selftest]
 */

import { mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(fileURLToPath(new URL("..", import.meta.url)));
const IGNORED_DIRECTORIES = new Set(["node_modules", "target", ".git", "dist"]);
const IGNORED_EXTENSIONS = new Set([
  ".node",
  ".png",
  ".jpg",
  ".jpeg",
  ".gif",
  ".webp",
  ".ico",
  ".pdf",
  ".zip",
  ".gz",
]);

// Split so this file never matches its own rule.
const PEM_HEADER = ["-----BEGIN", "(?:RSA |EC |OPENSSH |DSA |PGP )?PRIVATE KEY-----"].join(" ");

const RULES = [
  { name: "pem private key", pattern: new RegExp(PEM_HEADER) },
  { name: "aws access key id", pattern: /\b(?:AKIA|ASIA)[0-9A-Z]{16}\b/ },
  { name: "github token", pattern: /\bgh[pousr]_[A-Za-z0-9]{36,}\b/ },
  { name: "slack token", pattern: /\bxox[baprs]-[A-Za-z0-9-]{10,}\b/ },
  { name: "google api key", pattern: /\bAIza[0-9A-Za-z_-]{35}\b/ },
  { name: "stripe secret key", pattern: /\bsk_live_[0-9A-Za-z]{20,}\b/ },
];

// Only leaf keys, only under the TLS fixture directory, and only the PEM rule.
const FIXTURE_LEAF_KEY = /^tests\/fixtures\/tls\/[a-z-]+\.key$/;

function isFixtureLeafKey(relativePath) {
  return FIXTURE_LEAF_KEY.test(relativePath);
}

function scanText(relativePath, text) {
  const findings = [];
  for (const rule of RULES) {
    if (rule.name === "pem private key" && isFixtureLeafKey(relativePath)) continue;
    if (rule.pattern.test(text)) findings.push(`${relativePath}: ${rule.name}`);
  }
  return findings;
}

function walk(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      return IGNORED_DIRECTORIES.has(entry.name) ? [] : walk(path);
    }
    return [path];
  });
}

function scanRepository() {
  const findings = [];
  for (const path of walk(ROOT)) {
    const name = path.slice(path.lastIndexOf("."));
    if (IGNORED_EXTENSIONS.has(name)) continue;

    let text;
    try {
      text = readFileSync(path, "utf8");
    } catch {
      continue;
    }
    findings.push(...scanText(relative(ROOT, path).split(sep).join("/"), text));
  }
  return findings;
}

function selftest() {
  const fakeAws = `AKIA${"ABCDEFGHIJKLMNOP"}`;
  const fakePem = [
    ["-----BEGIN", "PRIVATE KEY-----"].join(" "),
    "MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE",
    ["-----END", "PRIVATE KEY-----"].join(" "),
  ].join("\n");

  const cases = [
    { path: "src/config.ts", text: `key = "${fakeAws}"`, expect: 1 },
    { path: "crates/sendust-core/src/config/mod.rs", text: fakePem, expect: 1 },
    { path: "tests/fixtures/tls/valid-server.key", text: fakePem, expect: 0 },
    { path: "tests/fixtures/other/valid-server.key", text: fakePem, expect: 1 },
    { path: "README.md", text: "nothing to see here", expect: 0 },
  ];

  const scratch = mkdtempSync(join(tmpdir(), "sendust-secret-scan-"));
  try {
    writeFileSync(join(scratch, "leak.env"), `AWS_ACCESS_KEY_ID=${fakeAws}\n`);
    writeFileSync(join(scratch, "leak.pem"), fakePem);

    let failed = 0;
    for (const testCase of cases) {
      const found = scanText(testCase.path, testCase.text).length;
      if (found !== testCase.expect) {
        failed += 1;
        console.error(`selftest FAIL ${testCase.path}: expected ${testCase.expect}, found ${found}`);
      }
    }
    if (failed > 0) {
      console.error(`check-secrets: ${failed} selftest case(s) failed`);
      process.exit(1);
    }
    console.log(`PASS: secret scan detects credentials and exempts only TLS fixture leaf keys`);
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

if (process.argv.includes("--selftest")) {
  selftest();
} else {
  const findings = scanRepository();
  if (findings.length > 0) {
    console.error(`check-secrets: ${findings.length} potential secret(s)`);
    for (const finding of findings) console.error(`  - ${finding}`);
    process.exit(1);
  }
  console.log("PASS: no credential shapes outside the TLS fixture leaf keys");
}
