/**
 * Autonomous delivery engine.
 *
 * Owns the queue, the pool of native transports, adaptive concurrency, the rate
 * limiter, retry scheduling, and shutdown. The native layer keeps its own
 * guarantee — one SMTP session, one send at a time — so nothing here may run two
 * transactions over one native transport.
 *
 * A caller's `send()` promise settles with the relay outcome for *that* message,
 * or with a typed error. No path drops a message without settling it.
 */

import { SendustClosedError, SendustError, SendustLimitError, fromNative } from "./errors.ts";
import { retryAfterError, retryAfterResult, type BackoffPolicy } from "./policy.ts";
import type { NativeTransport } from "./loader.ts";
import { toSendResult } from "./results.ts";
import type { SendResult, TransportState, TransportStats } from "./types.ts";

/** Everything the engine needs. `Transport` builds it from the caller's options. */
export interface EngineOptions {
  readonly maxConcurrent: number;
  readonly maxConnections: number;
  readonly maxQueue: number;
  readonly maxQueueBytes: number;
  readonly messagesPerSecond: number;
  readonly maxAttempts: number;
  readonly retryBaseMs: number;
  readonly retryMaxMs: number;
  readonly closeMs: number;
  /** Whole-message budget across every attempt, so retries cannot multiply it. */
  readonly totalMs: number;
  /**
   * How long a native session may sit idle before the pool releases it. The
   * core refuses to reuse a session past the same window; enforcing it here as
   * well is what actually closes the socket.
   */
  readonly sessionIdleMs: number;
  /** Creates one more native transport. Called only while the pool may grow. */
  readonly createTransport: () => NativeTransport;
  /** The transport created eagerly by `Transport`, so construction validates. */
  readonly first?: NativeTransport;
}

interface Job {
  readonly nativeMessage: Record<string, unknown>;
  readonly bytes: number;
  readonly resolve: (result: SendResult) => void;
  readonly reject: (error: unknown) => void;
  /** Wall-clock budget shared by every attempt of this message. */
  readonly deadline: number;
  attempts: number;
}

interface Retry {
  readonly job: Job;
  readonly timer: unknown;
}

interface Waiter {
  readonly resolve: (transport: NativeTransport) => void;
  readonly reject: (error: unknown) => void;
}

interface Counters {
  accepted: number;
  rejected: number;
  unknown: number;
  retried: number;
  queueRejections: number;
}

/** The automatic ceiling on concurrent sends, whatever the caller allows. */
const MAX_CONCURRENCY = 32;
/** How many consecutive successes raise concurrency by one. */
const GROWTH_STREAK = 8;
/** How long to hold concurrency down after throttling, before growing again. */
const COOLDOWN_MS = 1_000;

/**
 * Timer host, read off `globalThis` so this module needs no runtime-specific
 * type package: Node.js, Bun, and Deno all provide these functions.
 */
const timers = globalThis as unknown as {
  setTimeout: (handler: () => void, ms: number) => unknown;
  clearTimeout: (handle: unknown) => void;
};

function closedError(): SendustClosedError {
  return new SendustClosedError({
    code: "TRANSPORT_CLOSED",
    category: "closed",
    message: "transport is closed",
  });
}

function queueFullError(): SendustLimitError {
  return new SendustLimitError({
    code: "LIMIT_QUEUE_FULL",
    category: "limit",
    message: "delivery queue is at its configured bound",
  });
}

/**
 * One delivery engine per transport.
 *
 * Concurrency starts at one and grows only after sustained success, so a caller
 * who sends two messages is not suddenly opening eight connections; a relay that
 * throttles halves it immediately.
 */
export class DeliveryEngine {
  readonly #options: EngineOptions;
  readonly #backoff: BackoffPolicy;

  #state: TransportState = "open";
  #idle: NativeTransport[] = [];
  readonly #all = new Set<NativeTransport>();
  #waiters: Waiter[] = [];

  #queue: Job[] = [];
  #queueBytes = 0;
  #active = 0;
  #concurrency = 1;

  #streak = 0;
  #cooldownUntil = 0;
  #tokens = 1;
  #rateStamp = 0;
  #rateTimer: unknown;
  #idleTimer: unknown;

  readonly #retries = new Set<Retry>();
  #closePromise: Promise<void> | undefined;
  #fatal: SendustError | null = null;

  readonly #counters: Counters = {
    accepted: 0,
    rejected: 0,
    unknown: 0,
    retried: 0,
    queueRejections: 0,
  };
  constructor(options: EngineOptions) {
    this.#options = options;
    this.#backoff = { baseMs: options.retryBaseMs, maxMs: options.retryMaxMs };
    // One token, never a full bucket: a burst still has to earn its throughput,
    // so a configured rate is honored from the first message.
    this.#tokens = options.messagesPerSecond > 0 ? 1 : 0;
    if (options.first) {
      this.#all.add(options.first);
      this.#idle.push(options.first);
    }
  }

  get state(): TransportState {
    return this.#state;
  }

  get stats(): TransportStats {
    return {
      state: this.#state,
      queueDepth: this.#queue.length,
      queueBytes: this.#queueBytes,
      active: this.#active,
      connections: this.#all.size,
      concurrency: this.#concurrency,
      accepted: this.#counters.accepted,
      rejected: this.#counters.rejected,
      unknown: this.#counters.unknown,
      retried: this.#counters.retried,
      queueRejections: this.#counters.queueRejections,
    };
  }

  /**
   * The error a refused send reports.
   *
   * A failed engine names the cause that stopped it, so a caller who retries
   * after an auth failure is told `AUTH_FAILED` rather than `TRANSPORT_CLOSED`,
   * which would suggest the transport is merely shutting down.
   */
  get refusal(): SendustError {
    return this.#state === "failed" && this.#fatal ? this.#fatal : closedError();
  }

  /**
   * Queues one message. The returned promise settles with its own outcome.
   *
   * The native message record is already built, so the caller's buffers were
   * copied before this call; the engine never holds a caller-owned object.
   */
  submit(nativeMessage: Record<string, unknown>, bytes: number): Promise<SendResult> {
    if (this.#state !== "open") {
      return Promise.reject(this.refusal);
    }
    if (
      this.#queue.length >= this.#options.maxQueue ||
      this.#queueBytes + bytes > this.#options.maxQueueBytes
    ) {
      this.#counters.queueRejections += 1;
      return Promise.reject(queueFullError());
    }

    return new Promise<SendResult>((resolve, reject) => {
      this.#queue.push({
        nativeMessage,
        bytes,
        resolve,
        reject,
        deadline: Date.now() + this.#options.totalMs,
        attempts: 0,
      });
      this.#queueBytes += bytes;
      this.#pump();
    });
  }

  /**
   * Stops accepting work, rejects everything still waiting, and closes every
   * native transport.
   *
   * The policy is abort-on-close: queued messages fail with `TRANSPORT_CLOSED`,
   * an active send is cancelled by closing its native transport (which reports
   * `unknown` when the body may already have reached the relay), and every
   * promise settles. Idempotent.
   */
  close(): Promise<void> {
    this.#closePromise ??= this.#closeImpl();
    return this.#closePromise;
  }

  async #closeImpl(): Promise<void> {
    // Committed synchronously: a send that starts now is refused as CLOSED.
    this.#state = "closing";

    const error = closedError();
    for (const entry of this.#retries) {
      timers.clearTimeout(entry.timer);
      entry.job.reject(error);
    }
    this.#retries.clear();

    for (const job of this.#queue.splice(0)) {
      this.#queueBytes -= job.bytes;
      job.reject(error);
    }

    for (const waiter of this.#waiters.splice(0)) {
      waiter.reject(error);
    }

    if (this.#rateTimer !== undefined) {
      timers.clearTimeout(this.#rateTimer);
      this.#rateTimer = undefined;
    }

    if (this.#idleTimer !== undefined) {
      timers.clearTimeout(this.#idleTimer);
      this.#idleTimer = undefined;
    }

    // `closing` until every native session has finished closing, so a caller
    // that observes the state during shutdown sees the transition, not a jump.
    const transports = [...this.#all];
    this.#all.clear();
    this.#idle.length = 0;
    await Promise.allSettled(transports.map((transport) => transport.close()));
    this.#state = "closed";
  }

  // ---------------------------------------------------------------- scheduling

  #pump(): void {
    if (this.#state !== "open") {
      return;
    }
    this.#meetDemand();
    while (this.#active < this.#concurrency && this.#queue.length > 0) {
      if (!this.#allow()) {
        this.#scheduleRateWake();
        return;
      }
      const job = this.#queue.shift();
      if (!job) {
        return;
      }
      this.#queueBytes -= job.bytes;
      this.#active += 1;
      void this.#run(job);
    }
  }

  async #run(job: Job): Promise<void> {
    let transport: NativeTransport;
    try {
      transport = await this.#acquire();
    } catch (error) {
      this.#active -= 1;
      this.#fail(job, error);
      this.#pump();
      return;
    }

    job.attempts += 1;
    let accepted = false;
    try {
      const raw = await transport.send(job.nativeMessage);

      const result = toSendResult(raw);
      const decision = retryAfterResult(result, this.#ledger(job), this.#backoff);
      if (decision.retry && this.#budgetAllows(job, decision.delayMs)) {
        this.#counters.retried += 1;
        this.#throttle();
        this.#scheduleRetry(job, decision.delayMs);
      } else {
        accepted = result.status === "accepted";
        this.#count(result);
        job.resolve(result);
      }
    } catch (error) {
      const mapped = fromNative(error) ?? error;
      const decision = retryAfterError(mapped, this.#ledger(job), this.#backoff);
      const fatal = decision.retry === false && decision.fatal === true;
      if (decision.retry && this.#budgetAllows(job, decision.delayMs)) {
        this.#counters.retried += 1;
        this.#throttle();
        this.#scheduleRetry(job, decision.delayMs);
      } else {
        if (fatal && mapped instanceof SendustError) {
          this.#abortWith(mapped);
        }
        this.#fail(job, mapped);
      }
    } finally {
      // Released exactly once, on every exit. `toSendResult` can throw after a
      // send that succeeded, and releasing again in the catch path would hand
      // that same session to a waiter already using it.
      this.#release(transport);
      this.#active -= 1;
      if (accepted) {
        this.#grow();
      }
      this.#pump();
    }
  }

  #ledger(job: Job): { attempt: number; maxAttempts: number } {
    return { attempt: job.attempts, maxAttempts: this.#options.maxAttempts };
  }

  /**
   * A retry may only start while this message still has budget left, so a
   * stalled relay cannot turn one message into attempt-count × the timeout.
   */
  #budgetAllows(job: Job, delayMs: number): boolean {
    return this.#state === "open" && Date.now() + delayMs < job.deadline;
  }

  #scheduleRetry(job: Job, delayMs: number): void {
    let entry: Retry | undefined;
    const timer = timers.setTimeout(() => {
      if (entry) {
        this.#retries.delete(entry);
      }
      if (this.#state !== "open") {
        job.reject(closedError());
        return;
      }
      // Appended, never pushed to the front: a retry must not starve new work.
      this.#queue.push(job);
      this.#queueBytes += job.bytes;
      this.#pump();
    }, delayMs);
    entry = { job, timer };
    this.#retries.add(entry);
  }

  // --------------------------------------------------------------- adaptation

  /**
   * Raises concurrency to the work already waiting.
   *
   * A burst of queued messages should use the whole pool at once: growing one
   * slot per eight successes made a 100-message burst spend most of its run
   * under-provisioned, which showed up as a long tail on a real relay.
   * `#throttle` still halves the level and holds a cooldown after any temporary
   * failure, so a misbehaving relay cannot turn this into a runaway.
   */
  #meetDemand(): void {
    if (Date.now() < this.#cooldownUntil) {
      return;
    }
    const wanted = Math.min(this.#options.maxConcurrent, this.#active + this.#queue.length);
    if (wanted > this.#concurrency) {
      this.#concurrency = wanted;
    }
  }

  #grow(): void {
    this.#streak += 1;
    if (Date.now() < this.#cooldownUntil) {
      return;
    }
    if (this.#streak >= GROWTH_STREAK && this.#concurrency < this.#options.maxConcurrent) {
      this.#concurrency = Math.min(this.#options.maxConcurrent, this.#concurrency + 1);
      this.#streak = 0;
      this.#pump();
    }
  }

  #throttle(): void {
    this.#streak = 0;
    this.#cooldownUntil = Date.now() + COOLDOWN_MS;
    this.#concurrency = Math.max(1, Math.floor(this.#concurrency / 2));
  }

  #count(result: SendResult): void {
    if (result.status === "accepted") {
      this.#counters.accepted += 1;
    } else if (result.status === "rejected") {
      this.#counters.rejected += 1;
    } else {
      this.#counters.unknown += 1;
    }
  }

  #fail(job: Job, error: unknown): void {
    job.reject(error);
  }

  /**
   * Bad credentials cannot recover by waiting, so the whole transport stops:
   * queued work and scheduled retries fail fast with the same error instead of
   * hammering the relay, and later sends reject without I/O.
   *
   * The state moves to `failed` in the same synchronous step, so a health check
   * that reads `stats.state` after the rejected send does not see `open` while
   * every send fails.
   */
  #abortWith(error: SendustError): void {
    if (this.#fatal) {
      return;
    }
    this.#fatal = error;
    this.#state = "failed";
    for (const job of this.#queue.splice(0)) {
      this.#queueBytes -= job.bytes;
      job.reject(error);
    }
    for (const entry of this.#retries) {
      timers.clearTimeout(entry.timer);
      entry.job.reject(error);
    }
    this.#retries.clear();
  }

  // -------------------------------------------------------------- rate limiter

  #allow(): boolean {
    const perSecond = this.#options.messagesPerSecond;
    if (perSecond <= 0) {
      return true;
    }
    const now = Date.now();
    if (this.#rateStamp === 0) {
      this.#rateStamp = now;
    }
    const elapsed = now - this.#rateStamp;
    if (elapsed > 0) {
      this.#tokens = Math.min(perSecond, this.#tokens + (elapsed * perSecond) / 1000);
      this.#rateStamp = now;
    }
    if (this.#tokens >= 1) {
      this.#tokens -= 1;
      return true;
    }
    return false;
  }

  #scheduleRateWake(): void {
    if (this.#rateTimer !== undefined) {
      return;
    }
    this.#rateTimer = timers.setTimeout(() => {
      this.#rateTimer = undefined;
      this.#pump();
    }, 20);
  }

  // ---------------------------------------------------------------- connection

  async #acquire(): Promise<NativeTransport> {
    const idle = this.#idle.pop();
    if (idle) {
      return idle;
    }
    if (this.#all.size < Math.min(this.#options.maxConnections, MAX_CONCURRENCY)) {
      const transport = this.#options.createTransport();
      this.#all.add(transport);
      return transport;
    }
    return new Promise<NativeTransport>((resolve, reject) => {
      this.#waiters.push({ resolve, reject });
    });
  }

  #release(transport: NativeTransport): void {
    if (this.#state === "failed") {
      // A failed engine can never send again, so the session is closed now
      // rather than left open until the caller gets round to `close()`.
      this.#all.delete(transport);
      void transport.close().catch(() => undefined);
      return;
    }
    // Only an open engine re-pools a transport. While closing, `#closeImpl`
    // owns every session it captured and awaits its close.
    if (this.#state !== "open") {
      return;
    }
    const waiter = this.#waiters.shift();
    if (waiter) {
      waiter.resolve(transport);
      return;
    }
    if (this.#options.sessionIdleMs <= 0) {
      // Zero means no idle reuse, which is the rule the core applies: the
      // session is closed now rather than pooled. Pooling it would leave the
      // core refusing a session this engine keeps handing out.
      this.#all.delete(transport);
      void transport.close().catch(() => undefined);
      return;
    }
    this.#idle.push(transport);
    this.#armIdleRetire();
  }

  /**
   * Releases idle native sessions once the reuse window ends.
   *
   * The core already refuses to reuse a stale session, but refusing is not the
   * same as releasing: without this, a transport that sends one message and
   * lives on would hold an authenticated relay connection open until the next
   * send or `close()`. Retiring closes it with a polite `QUIT` and drops it
   * from the pool, so a later message dials a fresh session.
   */
  #armIdleRetire(): void {
    if (this.#state !== "open" || this.#idleTimer !== undefined) {
      return;
    }
    if (this.#idle.length === 0 || this.#options.sessionIdleMs <= 0) {
      return;
    }
    this.#idleTimer = timers.setTimeout(() => {
      this.#idleTimer = undefined;
      this.#retireIdle();
    }, this.#options.sessionIdleMs);
  }

  #retireIdle(): void {
    if (this.#state !== "open" || this.#idle.length === 0) {
      return;
    }
    for (const transport of this.#idle.splice(0)) {
      this.#all.delete(transport);
      // Best effort: the message this session served has already settled, so a
      // failed shutdown is not a caller's problem.
      void transport.close().catch(() => undefined);
    }
  }
}
