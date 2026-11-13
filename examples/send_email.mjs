// Sends one real message through the staged Node-API addon.
//
// Usage: npm run build:native && node examples/send_email.mjs
//
// SENDUST_HOST/PORT/SECURITY/FROM/TO/SUBJECT/TEXT are required as needed;
// SENDUST_USER and SENDUST_PASS are optional. Credentials are read from the
// environment and never written anywhere by this script.

import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const native = require("../ts/sendust.node");

const env = process.env;
if (!env.SENDUST_HOST || !env.SENDUST_FROM || !env.SENDUST_TO) {
  console.error("set SENDUST_HOST, SENDUST_FROM, and SENDUST_TO first");
  process.exit(2);
}

const options = {
  host: env.SENDUST_HOST,
  port: Number(env.SENDUST_PORT ?? "465"),
  secure: env.SENDUST_SECURITY ?? "tls",
};
if (env.SENDUST_USER) {
  options.auth = { user: env.SENDUST_USER, pass: env.SENDUST_PASS ?? "" };
}

const transport = new native.Transport(options);
try {
  const result = await transport.send({
    from: env.SENDUST_FROM,
    to: (env.SENDUST_TO ?? "")
      .split(",")
      .map((value) => value.trim())
      .filter(Boolean),
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
