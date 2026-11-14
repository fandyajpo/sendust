// Sends one real message through the staged Node-API addon.
//
// Usage: npm run build:native && node examples/send_email.mjs
//
// Configuration comes from `.env` in the repository root, or from real
// environment variables, which win. Start from the committed `.env.example`;
// its placeholder values are refused here, so an unedited copy cannot send.
//
// Credentials are read from the environment and never written anywhere by this
// script. A successful result means the relay accepted responsibility, not that
// a mailbox received the message.

import { readFileSync } from "node:fs";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const native = require("../ts/sendust.node");

// Load the committed example config; real environment variables win.
try {
  const file = readFileSync(new URL("../.env", import.meta.url), "utf8");
  for (const line of file.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const index = trimmed.indexOf("=");
    if (index === -1) continue;
    const key = trimmed.slice(0, index).trim();
    const value = trimmed
      .slice(index + 1)
      .trim()
      .replace(/^["']|["']$/g, "");
    if (process.env[key] === undefined) process.env[key] = value;
  }
} catch {
  // No .env file is fine; the variables can come from the environment.
}

const env = process.env;
if (!env.SENDUST_HOST || !env.SENDUST_FROM || !env.SENDUST_TO) {
  console.error("set SENDUST_HOST, SENDUST_FROM, and SENDUST_TO first");
  process.exit(2);
}

const placeholders = /example\.com|your-name|your-16-char-app-password/i;
for (const key of [
  "SENDUST_HOST",
  "SENDUST_USER",
  "SENDUST_PASS",
  "SENDUST_FROM",
  "SENDUST_TO",
]) {
  if (env[key] && placeholders.test(env[key])) {
    console.error(`${key} still contains an example placeholder; edit .env (see .env.example)`);
    process.exit(2);
  }
}

const user = env.SENDUST_USER || "";
let password = env.SENDUST_PASS || "";
if (user && !password) {
  console.error("SENDUST_USER is set but SENDUST_PASS is empty; set the password or unset both");
  process.exit(2);
}
if (!user && password) {
  console.error("SENDUST_PASS is set but SENDUST_USER is empty; set the user or unset both");
  process.exit(2);
}

const host = env.SENDUST_HOST;
const port = Number(env.SENDUST_PORT ?? "465");
const secure = env.SENDUST_SECURITY ?? "tls";

if ((secure === "tls" && port === 587) || (secure === "starttls" && port === 465)) {
  console.warn(`warning: port ${port} is unusual for ${secure}; 465 is implicit TLS, 587 is STARTTLS`);
}

if (user && /gmail\.com$/i.test(host) && password.includes(" ")) {
  console.warn("note: removed spaces from the Gmail app password");
  password = password.replace(/ /g, "");
}

const options = { host, port, secure };
if (user) {
  options.auth = { user, pass: password };
}

const recipients = (env.SENDUST_TO ?? "")
  .split(",")
  .map((value) => value.trim())
  .filter(Boolean);

console.log(`relay: ${host}:${port} ${secure}, auth: ${user ? "yes" : "no"}`);
console.log(`from: ${env.SENDUST_FROM}`);
console.log(`to: ${recipients.join(", ")}`);

const transport = new native.Transport(options);
try {
  const result = await transport.send({
    from: env.SENDUST_FROM,
    to: recipients,
    subject: env.SENDUST_SUBJECT ?? "Hello from Sendust",
    text: env.SENDUST_TEXT ?? "Sent through the Sendust Node-API binding.",
  });

  console.log(`status: ${result.status}`);
  for (const recipient of result.recipients) {
    console.log(`  ${recipient.address}: ${recipient.status}`);
  }
  if (result.response) {
    console.log(`relay: ${result.response.code} ${result.response.message}`);
  }
} finally {
  await transport.close();
}
