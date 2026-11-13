// Stages the compiled Node-API addon as ts/sendust.node for local use and CI.
import { copyFileSync, mkdirSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const release = process.env.SENDUST_DEBUG !== "1";

const args = ["build", "-p", "sendust-napi", "--locked"];
if (release) {
  args.push("--release");
}

execFileSync("cargo", args, { cwd: root, stdio: "inherit" });

const extension =
  process.platform === "darwin" ? "dylib" : process.platform === "win32" ? "dll" : "so";
const profile = release ? "release" : "debug";
const artifact = join(root, "target", profile, `libsendust_napi.${extension}`);
const destination = join(root, "ts", "sendust.node");

mkdirSync(dirname(destination), { recursive: true });
copyFileSync(artifact, destination);
console.log(`staged ${destination}`);
