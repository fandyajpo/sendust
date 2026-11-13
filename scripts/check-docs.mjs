#!/usr/bin/env node
/**
 * Static documentation guard for the Sendust repository.
 *
 * Verifies relative Markdown links, task registry <-> phase plan parity, ADR
 * and phase index coverage, governance document presence, and that the
 * toolchain and dependency pins declared in the source-of-truth documents are
 * actually enforced by the files that must carry them.
 */

import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(fileURLToPath(new URL("..", import.meta.url)));
const IGNORED_DIRECTORIES = new Set(["target", "node_modules", ".git", "dist"]);
const REQUIRED_TASKS = 95;
const TASKS_PER_PHASE = 5;

const REQUIRED_FILES = [
  "AGENTS.md",
  "README.md",
  "Cargo.toml",
  "rust-toolchain.toml",
  "package.json",
  "ts/package.json",
  "docs/README.md",
  "docs/engineering/toolchain-baseline.md",
  "docs/engineering/ai-implementation-contract.md",
  "docs/engineering/dependency-policy.md",
  "docs/planning/task-registry.md",
  "docs/planning/quality-gates.md",
  "docs/planning/definition-of-done.md",
  "docs/gates/phase-03-repository-baseline.md",
  "docs/architecture/decisions/README.md",
];

const failures = [];
const fail = (message) => failures.push(message);
const posix = (path) => relative(ROOT, path).split(sep).join("/");
const read = (path) => readFileSync(join(ROOT, path), "utf8");

function walk(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      return IGNORED_DIRECTORIES.has(entry.name) ? [] : walk(path);
    }
    return [path];
  });
}

function checkLinks() {
  const files = [
    ...walk(join(ROOT, "docs")).filter((path) => path.endsWith(".md")),
    join(ROOT, "README.md"),
    join(ROOT, "AGENTS.md"),
  ];
  for (const file of files) {
    const lines = readFileSync(file, "utf8").split("\n");
    let inFence = false;
    lines.forEach((line, index) => {
      if (/^\s*(```|~~~)/.test(line)) {
        inFence = !inFence;
        return;
      }
      if (inFence) return;
      for (const match of line.matchAll(/\[[^\]]*\]\(([^)\s]+)\)/g)) {
        const target = match[1];
        if (/^[a-zA-Z][a-zA-Z0-9+.-]*:/.test(target) || target.startsWith("#")) continue;
        const [pathPart] = target.split("#");
        if (!pathPart) continue;
        const resolved = join(dirname(file), decodeURIComponent(pathPart));
        if (!existsSync(resolved)) {
          fail(`${posix(file)}:${index + 1}: broken link -> ${target}`);
        }
      }
    });
  }
}

function normalizePrerequisites(value) {
  const trimmed = (value ?? "").trim();
  if (trimmed === "" || trimmed === "None" || trimmed === "—") return [];
  return trimmed
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function checkTaskRegistry() {
  const phaseFiles = readdirSync(join(ROOT, "docs/phases"))
    .filter((name) => /^phase-\d{2}\.md$/.test(name))
    .sort();
  const tasks = new Map();

  for (const name of phaseFiles) {
    const phase = name.slice(6, 8);
    const lines = read(`docs/phases/${name}`).split("\n");
    for (let index = 0; index < lines.length; index += 1) {
      const heading = /^### (P\d{2}-\d{3}) — (.+?)\s*$/.exec(lines[index]);
      if (!heading) continue;
      const [, id, title] = heading;
      let prerequisites = [];
      for (let cursor = index + 1; cursor < lines.length; cursor += 1) {
        if (lines[cursor].startsWith("### ")) break;
        const match = /^\s*(?:- )?\*\*Prerequisites:\*\* (.+?)\s*$/.exec(lines[cursor]);
        if (match) {
          prerequisites = normalizePrerequisites(match[1]);
          break;
        }
      }
      if (tasks.has(id)) fail(`duplicate task id ${id}`);
      tasks.set(id, { phase, title: title.trim(), prerequisites });
    }
  }

  if (tasks.size !== REQUIRED_TASKS) {
    fail(`expected ${REQUIRED_TASKS} phase tasks, found ${tasks.size}`);
  }

  const registry = new Map();
  for (const line of read("docs/planning/task-registry.md").split("\n")) {
    if (!/^\| \[P\d{2}\]\(/.test(line)) continue;
    const cells = line.split("|").map((cell) => cell.trim());
    const id = /^\[(P\d{2}-\d{3})\]/.exec(cells[3] ?? "")?.[1];
    if (!id) {
      fail(`task registry row has no task id: ${line}`);
      continue;
    }
    registry.set(id, {
      title: cells[4] ?? "",
      prerequisites: normalizePrerequisites(cells[6]),
    });
  }

  if (registry.size !== REQUIRED_TASKS) {
    fail(`expected ${REQUIRED_TASKS} task registry rows, found ${registry.size}`);
  }

  for (const [id, task] of tasks) {
    const row = registry.get(id);
    if (!row) {
      fail(`${id} is missing from docs/planning/task-registry.md`);
      continue;
    }
    if (row.title !== task.title) {
      fail(`${id} title differs: phase plan "${task.title}" vs registry "${row.title}"`);
    }
    const planned = [...task.prerequisites].sort().join(",");
    const registered = [...row.prerequisites].sort().join(",");
    if (planned !== registered) {
      fail(`${id} prerequisites differ: phase plan [${planned}] vs registry [${registered}]`);
    }
  }
  for (const id of registry.keys()) {
    if (!tasks.has(id)) fail(`${id} is in the task registry but not in any phase plan`);
  }

  const perPhase = new Map();
  for (const task of tasks.values()) {
    perPhase.set(task.phase, (perPhase.get(task.phase) ?? 0) + 1);
  }
  for (const [phase, count] of perPhase) {
    if (count !== TASKS_PER_PHASE) {
      fail(`phase ${phase} has ${count} tasks, expected ${TASKS_PER_PHASE}`);
    }
  }
}

function checkDecisionIndex() {
  const directory = join(ROOT, "docs/architecture/decisions");
  const files = readdirSync(directory)
    .filter((name) => /^\d{4}-.+\.md$/.test(name))
    .sort();
  const listed = new Set(
    [...read("docs/architecture/decisions/README.md").matchAll(/\((\d{4}-[^)]+\.md)\)/g)].map(
      (match) => match[1],
    ),
  );
  for (const file of files) {
    if (!listed.has(file)) fail(`ADR ${file} is not listed in the ADR index`);
  }
  for (const file of listed) {
    if (!files.includes(file)) fail(`ADR index lists a missing file: ${file}`);
  }
}

function checkPhaseIndex() {
  const index = read("docs/planning/phase-index.md");
  const phaseFiles = readdirSync(join(ROOT, "docs/phases")).filter((name) =>
    /^phase-\d{2}\.md$/.test(name),
  );
  for (const name of phaseFiles) {
    if (!index.includes(`../phases/${name}`)) {
      fail(`phase index does not link docs/phases/${name}`);
    }
  }
}

function checkGateIndex() {
  const gates = read("docs/planning/quality-gates.md");
  const listed = [...gates.matchAll(/^\| (QG-[A-Z0-9]+) \|/gm)].map((match) => match[1]);
  const detailed = [...gates.matchAll(/^## (QG-[A-Z0-9]+) — /gm)].map((match) => match[1]);

  const duplicates = listed.filter((id, index) => listed.indexOf(id) !== index);
  if (duplicates.length > 0) {
    fail(`duplicate gate ids in the gate index: ${[...new Set(duplicates)].join(", ")}`);
  }
  for (const id of listed) {
    if (!detailed.includes(id)) fail(`gate ${id} is listed in the gate index but has no detail section`);
  }
  for (const id of detailed) {
    if (!listed.includes(id)) fail(`gate ${id} has a detail section but is missing from the gate index`);
  }

  for (const name of readdirSync(join(ROOT, "docs/phases")).filter((file) =>
    /^phase-\d{2}\.md$/.test(file),
  )) {
    const phase = name.slice(6, 8);
    if (!read(`docs/phases/${name}`).includes(`PHASE-${phase}`)) {
      fail(`docs/phases/${name} does not name its completion gate PHASE-${phase}`);
    }
  }
}

function readPin(baseline, pattern, label) {
  const match = pattern.exec(baseline);
  if (!match) {
    fail(`docs/engineering/toolchain-baseline.md: cannot read the ${label} pin`);
    return null;
  }
  return match[1];
}

function checkToolchainPins() {
  const baseline = read("docs/engineering/toolchain-baseline.md");
  const rust = readPin(baseline, /\|\s*Rust toolchain\s*\|\s*`([^`]+)`/, "Rust toolchain");
  const npm = readPin(baseline, /\|\s*Package manager\s*\|\s*npm `([^`]+)`/, "npm");
  const typescript = readPin(baseline, /\|\s*TypeScript\s*\|\s*`([^`]+)`/, "TypeScript");
  const biome = readPin(baseline, /\|\s*Biome\s*\|\s*`([^`]+)`/, "Biome");
  const vitest = readPin(baseline, /\|\s*Vitest\s*\|\s*`([^`]+)`/, "Vitest");
  const bun = readPin(baseline, /\|\s*Bun\s*\|\s*`([^`]+)`/, "Bun");
  const deno = readPin(baseline, /\|\s*Deno\s*\|\s*`([^`]+)`/, "Deno");
  const node = [...baseline.matchAll(/\|\s*Node\.js\s*\|\s*`([^`]+)`/g)].map((match) => match[1]);
  if (node.length === 0) fail("docs/engineering/toolchain-baseline.md: no Node.js pins found");

  const channel = /channel\s*=\s*"([^"]+)"/.exec(read("rust-toolchain.toml"))?.[1];
  if (rust && channel !== rust) {
    fail(`rust-toolchain.toml channel ${channel} does not match the baseline ${rust}`);
  }

  const devDependencies = JSON.parse(read("ts/package.json")).devDependencies ?? {};
  for (const [name, version] of [
    ["typescript", typescript],
    ["@biomejs/biome", biome],
    ["vitest", vitest],
  ]) {
    if (version && devDependencies[name] !== version) {
      fail(`ts/package.json devDependency ${name} must be exactly ${version}`);
    }
  }

  const workflow = read(".github/workflows/ci.yml");
  for (const version of node) {
    if (!workflow.includes(version)) fail(`.github/workflows/ci.yml must pin Node.js ${version}`);
  }
  for (const [label, version] of [
    ["Bun", bun],
    ["Deno", deno],
    ["npm", npm],
  ]) {
    if (version && !workflow.includes(version)) {
      fail(`.github/workflows/ci.yml must pin ${label} ${version}`);
    }
  }
}

function checkDependencyPins() {
  const pins = [
    ...read("docs/engineering/dependency-policy.md").matchAll(
      /^\|\s*`([a-z0-9-]+)`\s*\|\s*`=([0-9][^`]*)`/gm,
    ),
  ];
  if (pins.length === 0) fail("dependency policy declares no exact dependency pins");
  const cargoToml = read("Cargo.toml");
  for (const [, name, version] of pins) {
    if (!new RegExp(`^${name}\\s*=`, "m").test(cargoToml)) {
      fail(`Cargo.toml does not declare ${name}`);
    }
    if (!cargoToml.includes(`"=${version}"`)) {
      fail(`Cargo.toml does not pin ${name} to =${version} as the dependency policy declares`);
    }
  }
}

checkLinks();
checkTaskRegistry();
checkDecisionIndex();
checkPhaseIndex();
checkGateIndex();
checkToolchainPins();
checkDependencyPins();

for (const path of REQUIRED_FILES) {
  if (!existsSync(join(ROOT, path))) fail(`required file missing: ${path}`);
}

if (failures.length > 0) {
  console.error(`check-docs: ${failures.length} problem(s)`);
  for (const failure of failures) console.error(`  - ${failure}`);
  process.exit(1);
}

console.log(
  "PASS: links, task registry parity, ADR and phase indexes, governance files, toolchain and dependency pins",
);
