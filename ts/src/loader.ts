/**
 * Isolated native addon loading boundary.
 *
 * Only this module may touch runtime-specific resolution. Shared wrapper
 * modules must stay runtime-neutral for Node.js, Bun, and Deno, so the loader
 * is lazy and free of static `node:` imports.
 */

import { moduleUrl } from "./module-context.ts";

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
  cwd?(): string;
  getBuiltinModule?(id: string): unknown;
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

  const processRef = (globalThis as unknown as { process?: ProcessLike }).process;
  const nodeModule = processRef?.getBuiltinModule?.("node:module") as
    | { createRequire(url: string): RequireLike }
    | undefined;
  const createRequire = nodeModule?.createRequire;

  if (!createRequire) {
    throw unavailable("no require bridge on this runtime");
  }

  // The package build rewrites the placeholder to the module URL. Running from
  // the source tree (tests, examples) keeps the placeholder, so resolve against
  // the current package directory instead.
  let base = moduleUrl;
  const fromSourceTree = base.includes("MODULE_URL_PLACEHOLDER");
  if (fromSourceTree) {
    const nodeUrl = processRef?.getBuiltinModule?.("node:url") as
      | { pathToFileURL(path: string): { href: string } }
      | undefined;
    const nodePath = processRef?.getBuiltinModule?.("node:path") as
      | { join(...parts: string[]): string }
      | undefined;
    const cwd = processRef?.cwd?.();
    if (!nodeUrl || !nodePath || !cwd) {
      throw unavailable("no module URL and no working directory to derive one from");
    }
    base = nodeUrl.pathToFileURL(nodePath.join(cwd, "package.json")).href;
  }

  const require = createRequire(base);
  let binding: unknown;
  try {
    // Bundled at the package root by the package build; when running from the
    // source tree the base is the package directory itself.
    binding = require(fromSourceTree ? "./sendust.node" : "../sendust.node");
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
