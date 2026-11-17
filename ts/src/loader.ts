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

/** The bundled addon is missing or cannot be loaded on this platform. */
function unavailable(detail: string): Error {
  const error = new Error(
    `Sendust native addon is unavailable (${detail}). Install the package build for this platform; no fallback build is used.`,
  );
  // Tag the stable code without importing the public error class, which would
  // make the shared loader depend on the facade's error module.
  Object.defineProperty(error, "code", { value: "CONFIG_NATIVE_UNAVAILABLE", enumerable: true });
  Object.defineProperty(error, "sendustCategory", { value: "configuration", enumerable: true });
  return error;
}

/** Loads the addon that `npm run build:native` stages next to this package. */
export function loadNative(): NativeBinding {
  if (cached) {
    return cached;
  }

  const meta = import.meta as { url: string };
  const processRef = (globalThis as unknown as { process?: ProcessLike }).process;
  const createRequire = processRef?.getBuiltinModule?.("node:module")?.createRequire;

  if (!createRequire) {
    throw unavailable("no require bridge on this runtime");
  }

  const require = createRequire(meta.url);
  let binding: unknown;
  try {
    // Bundled at the package root by the package build.
    binding = require("../sendust.node");
  } catch (error) {
    // Only a missing addon file is a packaging fault. Anything else (a broken
    // module, a wrong ABI) is rethrown untouched so it is not misdiagnosed.
    const code = (error as { code?: unknown } | null)?.code;
    if (code === "MODULE_NOT_FOUND") {
      throw unavailable("sendust.node not found for this platform");
    }
    throw error;
  }

  cached = binding as NativeBinding;
  return cached;
}
