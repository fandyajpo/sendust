#!/usr/bin/env node
/**
 * Documentation accuracy guard for the user guides.
 *
 * `check-docs.mjs` proves structure and links. This proves content:
 *
 * - every stable error code named in a guide exists in the error model;
 * - the cap and timeout numbers in the limits guide match the code;
 * - no guide claims delivery, automatic retries, pooling, SMTPUTF8, URL
 *   attachments, or a way to disable verification (negated statements are fine);
 * - a guide that shows a send also shows an explicit close.
 *
 * Usage: node scripts/check-doc-claims.mjs
 */

import { readdirSync, readFileSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(fileURLToPath(new URL("..", import.meta.url)));
const read = (path) => readFileSync(join(ROOT, path), "utf8");
const posix = (path) => relative(ROOT, join(ROOT, path)).split(sep).join("/");

const failures = [];
const fail = (message) => failures.push(message);

/** Guides plus the two entry-point READMEs carry the user-facing claims. */
function guides() {
  return [
    ...readdirSync(join(ROOT, "docs", "guides"))
      .filter((name) => name.endsWith(".md"))
      .map((name) => join("docs", "guides", name)),
    "README.md",
    "ts/README.md",
  ];
}

// 1. Error codes must exist in the normative catalog.
// Only the documented prefixes are treated as codes, so identifiers such as
// `PLATFORM_PACKAGES` or `SENDUST_HOST` are not mistaken for error codes.
const CODE = /`((?:VALIDATION|CONFIG|LIMIT|TLS|AUTH|PROTOCOL|TIMEOUT|TRANSPORT)_[A-Z0-9_]+)`/g;

function knownCodes() {
  const model = read("docs/architecture/error-model.md");
  return new Set([...model.matchAll(/^\| `([A-Z][A-Z0-9]*(?:_[A-Z0-9]+)+)` \|/gm)].map((m) => m[1]));
}

function checkCodes() {
  const catalog = knownCodes();
  for (const file of guides()) {
    for (const match of read(file).matchAll(CODE)) {
      if (!catalog.has(match[1])) {
        fail(`${posix(file)}: \`${match[1]}\` is not in the error catalog`);
      }
    }
  }
}

// 2. Cap and timeout numbers in the limits guide must equal the code.
function evaluate(expression, miB) {
  return expression
    .split("*")
    .map((part) => part.trim())
    .reduce((total, part) => {
      if (part === "MIB") return total * miB;
      if (/^\d+$/.test(part)) return total * Number(part);
      return Number.NaN;
    }, 1);
}

function checkNumbers() {
  const limits = read("ts/src/limits.ts");
  const config = read("crates/sendust-core/src/config/mod.rs");

  const miB = evaluate(/export const MIB = ([^;]+);/.exec(limits)?.[1] ?? "", 1);
  const cap = (name) => evaluate(new RegExp(`${name}: ([^,]+),`).exec(limits)?.[1] ?? "", miB);
  const caps = {
    "10 MiB": cap("maxAttachmentBytes"),
    "20 MiB": cap("maxTotalAttachmentBytes"),
    "30 MiB": cap("maxWireBytes"),
  };

  const seconds = (field) =>
    Number(new RegExp(`${field}: Duration::from_secs\\((\\d+)\\)`).exec(config)?.[1]);
  const timeouts = {
    10_000: seconds("connect") * 1000,
    30_000: seconds("operation") * 1000,
    120_000: seconds("total") * 1000,
    125_000: seconds("close") * 1000,
  };

  if ([miB, ...Object.values(caps), ...Object.values(timeouts)].some((value) => Number.isNaN(value))) {
    fail("could not derive the caps or timeouts from the code");
    return;
  }
  if (Object.entries(timeouts).some(([expected, actual]) => actual !== Number(expected))) {
    fail("config/mod.rs: timeout defaults are not the documented 10/30/120/125 s");
  }

  const guide = read("docs/guides/lifecycle-limits.md");
  // Collapse grouped digits ("10 485 760") so one comparison covers formatting.
  const digits = guide.replace(/(\d)[\s,](?=\d)/g, "$1");

  for (const [label, value] of Object.entries(caps)) {
    if (!guide.includes(label)) {
      fail(`lifecycle-limits.md: missing the ${label} cap`);
    }
    if (!digits.includes(String(value))) {
      fail(`lifecycle-limits.md: missing the ${value} byte value for ${label}`);
    }
  }
  for (const value of Object.keys(timeouts)) {
    if (!digits.includes(value)) {
      fail(`lifecycle-limits.md: missing the ${value} ms timeout default`);
    }
  }
}

// 3. Forbidden claims, unless the sentence negates them.
const FORBIDDEN = [
  { pattern: /guarantees?\s+delivery/i, label: "delivery guarantee" },
  { pattern: /delivery\s+guarantee/i, label: "delivery guarantee" },
  { pattern: /automatically\s+retri/i, label: "automatic retry" },
  { pattern: /retri(?:es|ed)?\s+automatically/i, label: "automatic retry" },
  { pattern: /smtputf8\s+(?:is|are)\s+supported/i, label: "SMTPUTF8 support" },
  { pattern: /url\s+attachments?\s+(?:are|is)\s+supported/i, label: "URL attachment support" },
  { pattern: /disables?\s+(?:certificate\s+or\s+hostname\s+|tls\s+)?verification/i, label: "disabled verification" },
  { pattern: /connection\s+pooling\s+is\s+supported/i, label: "pooling support" },
];

function negated(text, index) {
  const before = text.slice(Math.max(0, index - 60), index).toLowerCase();
  return /\b(?:no|not|never|cannot|can't|without|isn't|doesn't|no way to)\b/.test(before);
}

function checkClaims() {
  for (const file of guides()) {
    const text = read(file);
    for (const { pattern, label } of FORBIDDEN) {
      for (const match of text.matchAll(new RegExp(pattern.source, "gi"))) {
        if (!negated(text, match.index ?? 0)) {
          fail(`${posix(file)}: unsupported ${label} claim ("${match[0]}")`);
        }
      }
    }
  }
}

// 4. A guide that shows a send must show an explicit close.
function checkClose() {
  for (const file of guides()) {
    const text = read(file);
    if (text.includes("transport.send(") && !text.includes("transport.close(")) {
      fail(`${posix(file)}: shows a send without an explicit close`);
    }
  }
}

checkCodes();
checkNumbers();
checkClaims();
checkClose();

if (failures.length > 0) {
  for (const message of failures) {
    console.error(`FAIL: ${message}`);
  }
  process.exit(1);
}
console.log("PASS: guide codes, cap/timeout numbers, forbidden claims, and explicit closes");
