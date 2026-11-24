import { readFileSync } from "node:fs";
import type { Socket } from "node:net";
import { dirname, join } from "node:path";
import { createServer as createTlsServer } from "node:tls";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { DeliveryEngine } from "../src/engine.ts";
import {
  SendustAuthenticationError,
  SendustError,
  SendustInternalError,
  createTransport,
  type DeliveryOptions,
  type Message,
  type SendResult,
  type Transport,
} from "../src/index.ts";
import type { NativeSendResult, NativeTransport } from "../src/loader.ts";

let available = true;
try {
  const { loadNative } = await import("../src/loader.ts");
  loadNative();
} catch {
  available = false;
}

/** Reads one committed TLS fixture relative to the repository root. */
function tlsFixture(name: string): string {
  const path = join(
    dirname(fileURLToPath(import.meta.url)),
    "..",
    "..",
    "tests",
    "fixtures",
    "tls",
  );
  return readFileSync(join(path, name), "utf8");
}

/** The final reply a fake connection gives after the DATA terminator. */
type Final = "accepted" | "transient" | "permanent" | "stall";

interface Relay {
  readonly port: number;
  readonly connections: () => number;
  readonly close: () => Promise<void>;
}

function message(index: number): Message {
  return {
    from: "sender@example.test",
    to: ["recipient@example.test"],
    subject: `message ${index}`,
    text: `hello ${index}`,
  };
}

function transportFor(relay: Relay, delivery?: DeliveryOptions): Transport {
  return createTransport({
    host: "127.0.0.1",
    port: relay.port,
    secure: "tls",
    tlsRootCertificate: tlsFixture("ca.crt"),
    // `exactOptionalPropertyTypes` forbids an explicit undefined here, so the
    // key is present only when the test configured it.
    ...(delivery ? { delivery } : {}),
  });
}

async function waitFor(predicate: () => boolean, timeoutMs = 5000): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (!predicate()) {
    if (Date.now() > deadline) {
      throw new Error("condition not reached");
    }
    await new Promise((resolve) => setTimeout(resolve, 5));
  }
}

/**
 * A loopback SMTP relay over TLS with the committed valid fixture. `behaviour`
 * decides the final reply for each *message* (not each connection), so a reused
 * session and a retried message are both modelled correctly.
 */
async function fakeRelay(behaviour: () => Final = () => "accepted"): Promise<Relay> {
  const sockets = new Set<Socket>();
  let connections = 0;

  const server = createTlsServer(
    { key: tlsFixture("valid-server.key"), cert: tlsFixture("valid-server.crt") },
    (socket: Socket) => {
      connections += 1;
      sockets.add(socket);
      socket.on("close", () => sockets.delete(socket));
      socket.setEncoding("utf8");
      socket.write("220 sendust.test ESMTP ready\r\n");

      let buffer = "";
      let inData = false;
      socket.on("data", (chunk: string) => {
        buffer += chunk;
        for (;;) {
          const index = buffer.indexOf("\r\n");
          if (index === -1) {
            return;
          }
          const line = buffer.slice(0, index);
          buffer = buffer.slice(index + 2);
          if (inData) {
            if (line === ".") {
              inData = false;
              const outcome = behaviour();
              if (outcome === "transient") {
                socket.write("451 4.3.0 try again later\r\n");
              } else if (outcome === "permanent") {
                socket.write("550 5.1.1 no such user\r\n");
              } else if (outcome === "accepted") {
                socket.write("250 2.0.0 accepted\r\n");
              }
              // `stall` deliberately writes nothing: the send stays in flight.
            }
            continue;
          }
          if (line.startsWith("EHLO")) {
            socket.write("250-sendust.test\r\n250 8BITMIME\r\n");
          } else if (line.startsWith("MAIL")) {
            socket.write("250 2.1.0 ok\r\n");
          } else if (line.startsWith("RCPT")) {
            socket.write("250 2.1.5 ok\r\n");
          } else if (line === "DATA") {
            inData = true;
            socket.write("354 go\r\n");
          } else if (line.startsWith("RSET")) {
            socket.write("250 2.0.0 ok\r\n");
          } else if (line.startsWith("QUIT")) {
            socket.write("221 2.0.0 bye\r\n");
            socket.end();
          }
        }
      });
    },
  );

  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const address = server.address();
  const port = typeof address === "object" && address ? address.port : 1;

  return {
    port,
    connections: () => connections,
    close: async () => {
      for (const socket of sockets) {
        socket.destroy();
      }
      await new Promise<void>((resolve) => server.close(() => resolve()));
    },
  };
}

/** The stable code of a settled promise, or null when it fulfilled. */
async function settleCode(promise: Promise<unknown>): Promise<string | null> {
  try {
    await promise;
    return null;
  } catch (error) {
    return error instanceof SendustError ? error.code : "unknown-error";
  }
}

/** A native transport stand-in, so a contract violation is provable without I/O. */
class FakeTransport implements NativeTransport {
  /** Sends this session is running at once; the pool must keep it at one. */
  inFlight = 0;
  /** The most it ever ran at once, which is how a double release shows. */
  overlapping = 0;
  calls = 0;
  closed = 0;
  readonly #respond: (index: number) => NativeSendResult;

  constructor(respond: (index: number) => NativeSendResult) {
    this.#respond = respond;
  }

  state(): string {
    return "open";
  }

  /** The native message record is opaque to the engine, so it is not read. */
  async send(): Promise<NativeSendResult> {
    this.calls += 1;
    this.inFlight += 1;
    this.overlapping = Math.max(this.overlapping, this.inFlight);
    const index = this.calls - 1;
    try {
      // A short turn, so two copies of this session in the idle list would
      // really overlap on the next two sends instead of serializing by luck.
      await new Promise((resolve) => setTimeout(resolve, 5));
      return this.#respond(index);
    } finally {
      this.inFlight -= 1;
    }
  }

  async close(): Promise<void> {
    this.closed += 1;
  }
}

/** A native payload the mapping rejects: one undocumented status is enough. */
function nativeOutcome(status: string): NativeSendResult {
  return { status, recipients: [{ address: "recipient@example.test", status }] };
}

/** A native rejection, shaped as the addon writes it (a redacted payload). */
function nativeFailure(code: string, category: string): Error {
  return new Error(`SENDUST:${JSON.stringify({ code, category, message: code })}`);
}

/** An engine whose pool is one stand-in session, with two slots of concurrency. */
function engineOver(transport: NativeTransport): DeliveryEngine {
  return new DeliveryEngine({
    maxConcurrent: 2,
    maxConnections: 1,
    maxQueue: 10,
    maxQueueBytes: 1 << 20,
    messagesPerSecond: 0,
    maxAttempts: 1,
    retryBaseMs: 0,
    retryMaxMs: 0,
    closeMs: 125_000,
    totalMs: 120_000,
    sessionIdleMs: 30_000,
    createTransport: () => {
      throw new Error("the pool must not grow past its one session");
    },
    first: transport,
  });
}

describe("autonomous delivery engine", () => {
  it.skipIf(!available)("accepts a burst of concurrent sends from one queue", async () => {
    const relay = await fakeRelay();
    const transport = transportFor(relay, {
      maxConcurrent: 8,
      maxConnections: 8,
      maxQueue: 1000,
    });
    try {
      const results = await Promise.all(
        Array.from({ length: 100 }, (_, index) => transport.send(message(index))),
      );
      expect(results).toHaveLength(100);
      expect(results.every((result: SendResult) => result.status === "accepted")).toBe(true);
      expect(transport.stats.accepted).toBe(100);
      expect(transport.stats.queueDepth).toBe(0);
      expect(transport.stats.connections).toBeLessThanOrEqual(8);
      // Sessions are reused: the pool dials at most once per connection, not
      // once per message.
      expect(relay.connections()).toBeLessThanOrEqual(8);
    } finally {
      await transport.close();
      await relay.close();
    }
  });

  it.skipIf(!available)("rejects with LIMIT_QUEUE_FULL when the bound is reached", async () => {
    const relay = await fakeRelay(() => "stall");
    const transport = transportFor(relay, { maxConcurrent: 1, maxQueue: 1 });
    try {
      const pending = Array.from({ length: 8 }, (_, index) =>
        settleCode(transport.send(message(index))),
      );
      await waitFor(() => relay.connections() >= 1);
      expect(transport.stats.queueRejections).toBeGreaterThan(0);

      await transport.close();
      const codes = await Promise.all(pending);
      expect(codes, JSON.stringify(codes)).toContain("LIMIT_QUEUE_FULL");
      expect(codes, JSON.stringify(codes)).toContain("TRANSPORT_CLOSED");
    } finally {
      await relay.close();
    }
  });

  it.skipIf(!available)("retries a transient 451 and reports the eventual acceptance", async () => {
    let deliveries = 0;
    const relay = await fakeRelay(() => (deliveries++ === 0 ? "transient" : "accepted"));
    const transport = transportFor(relay, {
      maxAttempts: 3,
      retryBaseMs: 0,
      retryMaxMs: 0,
    });
    try {
      const result = await transport.send(message(1));
      expect(result.status).toBe("accepted");
      expect(transport.stats.retried).toBe(1);
      expect(transport.stats.accepted).toBe(1);
      // The retry reuses the session that got the 451 (ADR-0022).
      expect(relay.connections()).toBe(1);
    } finally {
      await transport.close();
      await relay.close();
    }
  });

  it.skipIf(!available)("never retries a permanent 550", async () => {
    const relay = await fakeRelay(() => "permanent");
    const transport = transportFor(relay, { maxAttempts: 3, retryBaseMs: 0, retryMaxMs: 0 });
    try {
      const result = await transport.send(message(1));
      expect(result.status).toBe("rejected");
      expect(result.response?.code).toBe(550);
      expect(transport.stats.retried).toBe(0);
      expect(relay.connections()).toBe(1);
    } finally {
      await transport.close();
      await relay.close();
    }
  });

  it.skipIf(!available)("isolates a failed message from the rest of the batch", async () => {
    let deliveries = 0;
    const relay = await fakeRelay(() => (deliveries++ === 0 ? "permanent" : "accepted"));
    const transport = transportFor(relay, { maxConcurrent: 1, retryBaseMs: 0, retryMaxMs: 0 });
    try {
      const results = await Promise.all(
        Array.from({ length: 5 }, (_, index) => transport.send(message(index))),
      );
      const statuses = results.map((result) => result.status);
      expect(statuses.filter((status) => status === "rejected")).toHaveLength(1);
      expect(statuses.filter((status) => status === "accepted")).toHaveLength(4);
    } finally {
      await transport.close();
      await relay.close();
    }
  });

  it.skipIf(!available)("settles queued work with TRANSPORT_CLOSED on close", async () => {
    const relay = await fakeRelay(() => "stall");
    const transport = transportFor(relay, { maxConcurrent: 1, maxQueue: 20 });
    try {
      const pending = Array.from({ length: 5 }, (_, index) =>
        settleCode(transport.send(message(index))),
      );
      await waitFor(() => relay.connections() >= 1);
      await transport.close();
      const codes = await Promise.all(pending);
      // A queued message is refused; one already transmitting may resolve as
      // `unknown` because the body could have reached the relay. Neither hangs.
      expect(codes, JSON.stringify(codes)).toContain("TRANSPORT_CLOSED");
      expect(
        codes.every((code) => code === null || code === "TRANSPORT_CLOSED"),
        JSON.stringify(codes),
      ).toBe(true);
      expect(transport.state).toBe("closed");
      await expect(transport.send(message(99))).rejects.toBeInstanceOf(SendustError);
    } finally {
      await relay.close();
    }
  });

  it.skipIf(!available)("reuses one session across many sequential sends", async () => {
    const relay = await fakeRelay();
    const transport = transportFor(relay, {
      maxConcurrent: 1,
      maxConnections: 1,
      maxMessagesPerSession: 10_000,
    });
    try {
      for (let index = 0; index < 200; index += 1) {
        await transport.send(message(index));
      }
      expect(relay.connections()).toBe(1);
      expect(transport.stats.accepted).toBe(200);
    } finally {
      await transport.close();
      await relay.close();
    }
  });

  it.skipIf(!available)("opens a new session at the message cap", async () => {
    const relay = await fakeRelay();
    const transport = transportFor(relay, {
      maxConcurrent: 1,
      maxConnections: 1,
      maxMessagesPerSession: 50,
    });
    try {
      for (let index = 0; index < 120; index += 1) {
        await transport.send(message(index));
      }
      // 120 messages at 50 per session: 50, 50, then 20 on a third.
      expect(relay.connections()).toBe(3);
    } finally {
      await transport.close();
      await relay.close();
    }
  });

  it.skipIf(!available)("uses the whole pool as soon as work is waiting", async () => {
    // A stalled relay keeps every send in flight, so the pool's own level is
    // what is observable rather than how fast the relay answers.
    const relay = await fakeRelay(() => "stall");
    const transport = transportFor(relay, { maxConcurrent: 8, maxConnections: 8 });
    try {
      const pending = Array.from({ length: 8 }, (_, index) =>
        transport.send(message(index)).catch(() => undefined),
      );
      await waitFor(() => transport.stats.active === 8);
      // Demand sets the level, not a success streak: eight queued sends must
      // open eight sessions, which is what a real-relay burst needs.
      expect(transport.stats.concurrency).toBe(8);
      expect(transport.stats.connections).toBe(8);

      await transport.close();
      await Promise.all(pending);
    } finally {
      await relay.close();
    }
  });

  it.skipIf(!available)("spaces messages when a rate limit is configured", async () => {
    const relay = await fakeRelay();
    const transport = transportFor(relay, { messagesPerSecond: 20, maxConcurrent: 4 });
    try {
      const started = Date.now();
      const results = await Promise.all(
        Array.from({ length: 5 }, (_, index) => transport.send(message(index))),
      );
      const elapsed = Date.now() - started;
      expect(results.every((result) => result.status === "accepted")).toBe(true);
      // Five tokens at 20/s cannot all be spent at once.
      expect(elapsed).toBeGreaterThanOrEqual(100);
    } finally {
      await transport.close();
      await relay.close();
    }
  });

  it.skipIf(!available)("never reuses a session when sessionIdleMs is zero", async () => {
    const relay = await fakeRelay();
    const transport = transportFor(relay, { sessionIdleMs: 0, maxAttempts: 1 });
    try {
      await transport.send(message(1));
      await transport.send(message(2));
      // Zero is "no idle reuse", the rule the core applies too: the session
      // that served the first message is closed when it settles, so the second
      // message dials again.
      expect(relay.connections()).toBe(2);
    } finally {
      await transport.close();
      await relay.close();
    }
  });

  it("rejects an out-of-range delivery option before any I/O", () => {
    expect(() =>
      createTransport({
        host: "relay.test",
        port: 465,
        secure: "tls",
        delivery: { maxConcurrent: 0 },
      }),
    ).toThrow(SendustError);
    expect(() =>
      createTransport({
        host: "relay.test",
        port: 465,
        secure: "tls",
        delivery: { retryBaseMs: 500, retryMaxMs: 100 },
      }),
    ).toThrow(SendustError);
  });
});

describe("delivery engine contract seams", () => {
  it("releases a pool session once when the native result fails mapping", async () => {
    const transport = new FakeTransport(() => nativeOutcome("delivered"));
    const engine = engineOver(transport);
    try {
      const first = await engine.submit({}, 1).catch((error: unknown) => error);
      expect(first).toBeInstanceOf(SendustInternalError);
      expect((first as SendustError).code).toBe("INTERNAL");

      // Two more sends must not run at once on the pool's one session: a second
      // release would have left that same session in the idle list twice.
      await Promise.allSettled([engine.submit({}, 1), engine.submit({}, 1)]);
      expect(transport.overlapping).toBe(1);
      expect(transport.calls).toBe(3);
    } finally {
      await engine.close();
    }
  });

  it("reports a failed engine in state, and its cause on a later send", async () => {
    const transport = new FakeTransport(() => {
      throw nativeFailure("AUTH_FAILED", "authentication");
    });
    const engine = engineOver(transport);
    try {
      const first = await engine.submit({}, 1).catch((error: unknown) => error);
      expect(first).toBeInstanceOf(SendustAuthenticationError);

      // A health check reads this: it must not stay `open` while every send
      // fails, and `failed` says a new transport is the only recovery.
      expect(engine.state).toBe("failed");
      expect(engine.stats.state).toBe("failed");

      // The later send names the cause that stopped the engine, and dials
      // nothing.
      const later = await engine.submit({}, 1).catch((error: unknown) => error);
      expect((later as SendustError).code).toBe("AUTH_FAILED");
      expect(transport.calls).toBe(1);
    } finally {
      await engine.close();
    }

    // A failed engine can never send again, so the abandoned session was closed
    // when its send settled; closing the engine still works and settles closed.
    expect(transport.closed).toBe(1);
    expect(engine.state).toBe("closed");
  });
});
