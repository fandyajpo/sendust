/**
 * Isolated native addon loading boundary.
 *
 * Only this module may touch runtime-specific resolution. Shared wrapper
 * modules must stay runtime-neutral for Node.js, Bun, and Deno, so the loader
 * is lazy and free of static `node:` imports.
 */

export interface NativeSendResult {
  status: string;
  recipients: Array<{ address: string; status: string }>;
  response?: { code: number; message: string };
}

export interface NativeTransport {
  state(): string;
  send(message: Record<string, unknown>): Promise<NativeSendResult>;
  close(): Promise<void>;
}

export interface NativeBinding {
  Transport: new (options: Record<string, unknown>) => NativeTransport;
  nativeVersion(): string;
}

type RequireLike = (id: string) => unknown;

interface ProcessLike {
  getBuiltinModule?(id: string): { createRequire(url: string): RequireLike };
}

let cached: NativeBinding | undefined;

/** Loads the addon that `npm run build:native` stages next to this package. */
export function loadNative(): NativeBinding {
  if (cached) {
    return cached;
  }

  const meta = import.meta as { url: string };
  const processRef = (globalThis as unknown as { process?: ProcessLike }).process;
  const createRequire = processRef?.getBuiltinModule?.("node:module")?.createRequire;

  if (!createRequire) {
    throw new Error(
      "Sendust requires Node.js 22+ or another runtime that provides process.getBuiltinModule",
    );
  }

  const require = createRequire(meta.url);
  // Staged at the package root by `npm run build:native`.
  cached = require("../sendust.node") as NativeBinding;
  return cached;
}
