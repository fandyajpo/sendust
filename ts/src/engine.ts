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

import {
  SendustClosedError,
  SendustError,
  SendustLimitError,
  fromNative,
} from "./errors.ts";
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
  attempts: number;
}

interface Retry {
  readonly job: Job;
  readonly timer: ReturnType<typeof setTimeout>;
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
  #rateTimer: ReturnType<typeof setTimeout> | undefined;

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
   * Queues one message. The returned promise settles with its own outcome.
   *
   * The native message record is already built, so the caller's buffers were
   * copied before this call; the engine never holds a caller-owned object.
   */
  submit(nativeMessage: Record<string, unknown>, bytes: number): Promise<SendResult> {
    if (this.#state !== "open") {
      return Promise.reject(closedError());
    }
    if (this.#fatal) {
      return Promise.reject(this.#fatal);
    }
    if (
      this.#queue.length >= this.#options.maxQueue ||
      this.#queueBytes + bytes > this.#options.maxQueueBytes
    ) {
      this.#counters.queueRejections += 1;
      return Promise.reject(queueFullError());
    }

    return new Promise<SendResult>((resolve, reject) => {
      this.#queue.push({ nativeMessage, bytes, resolve, reject, attempts: 0 });
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
      clearTimeout(entry.timer);
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
      clearTimeout(this.#rateTimer);
      this.#rateTimer = undefined;
    }

    this.#state = "closed";

    await Promise.allSettled([...this.#all].map((transport) => transport.close()));
    this.#all.clear();
    this.#idle.length = 0;
  }

  // ---------------------------------------------------------------- scheduling

  #pump(): void {
    if (this.#state !== "open") {
      return;
    }
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
      this.#release(transport);

      const result = toSendResult(raw);
      const decision = retryAfterResult(result, this.#ledger(job), this.#backoff);
      if (decision.retry) {
        this.#counters.retried += 1;
        this.#throttle();
        this.#scheduleRetry(job, decision.delayMs);
      } else {
        accepted = result.status === "accepted";
        this.#count(result);
        job.resolve(result);
      }
    } catch (error) {
      this.#release(transport);

      const mapped = fromNative(error) ?? error;
      const decision = retryAfterError(mapped, this.#ledger(job), this.#backoff);
      if (decision.retry) {
        this.#counters.retried += 1;
        this.#throttle();
        this.#scheduleRetry(job, decision.delayMs);
      } else {
        if (decision.fatal === true && mapped instanceof SendustError) {
          this.#abortWith(mapped);
        }
        this.#fail(job, mapped);
      }
    } finally {
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

  #scheduleRetry(job: Job, delayMs: number): void {
    let entry: Retry | undefined;
    const timer = setTimeout(() => {
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
   */
  #abortWith(error: SendustError): void {
    if (this.#fatal) {
      return;
    }
    this.#fatal = error;
    for (const job of this.#queue.splice(0)) {
      this.#queueBytes -= job.bytes;
      job.reject(error);
    }
    for (const entry of this.#retries) {
      clearTimeout(entry.timer);
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
      this.#tokens = Math.min(
        perSecond,
        this.#tokens + (elapsed * perSecond) / 1000,
      );
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
    this.#rateTimer = setTimeout(() => {
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
    if (this.#state === "closed") {
      return;
    }
    const waiter = this.#waiters.shift();
    if (waiter) {
      waiter.resolve(transport);
    } else {
      this.#idle.push(transport);
    }
  }

}
