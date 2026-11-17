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
  platform?: string;
  arch?: string;
  report?: { getReport?(): { header?: { glibcVersionRuntime?: string } } };
}

/** Platform key to its optional native package (ADR-0020). */
const PLATFORM_PACKAGES: Record<string, string> = {
  "linux-x64-gnu": "@sendust/ts-linux-x64-gnu",
  "linux-arm64-gnu": "@sendust/ts-linux-arm64-gnu",
  "macos-x64": "@sendust/ts-macos-x64",
  "macos-arm64": "@sendust/ts-macos-arm64",
  "windows-x64": "@sendust/ts-windows-x64",
};

/**
 * The optional package for this platform, or `undefined` when the platform has
 * no build. `libc` only matters on Linux.
 */
export function platformPackageName(
  platform: string,
  arch: string,
  libc: "gnu" | "musl" = "gnu",
): string | undefined {
  const key =
    platform === "linux" ? `linux-${arch}-${libc}` : `${platform === "darwin" ? "macos" : platform}-${arch}`;
  return PLATFORM_PACKAGES[key];
}

/** glibc vs musl on Linux; every other platform is treated as gnu-agnostic. */
function linuxLibc(processRef: ProcessLike | undefined): "gnu" | "musl" {
  if (processRef?.platform !== "linux") {
    return "gnu";
  }
  try {
    const header = processRef.report?.getReport?.().header;
    return header?.glibcVersionRuntime ? "gnu" : "musl";
  } catch {
    return "gnu";
  }
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
  const platform = processRef?.platform ?? "unknown";
  const arch = processRef?.arch ?? "unknown";
  const packageName = platformPackageName(platform, arch, linuxLibc(processRef));

  let binding: unknown;
  try {
    // Bundled at the package root by the package build; when running from the
    // source tree the base is the package directory itself.
    binding = require(fromSourceTree ? "./sendust.node" : "../sendust.node");
  } catch (error) {
    if (!isMissingModule(error)) {
      // A broken module or a wrong ABI is rethrown untouched so it is not
      // misdiagnosed as a packaging fault.
      throw error;
    }

    // Optional per-platform package (ADR-0020).
    if (!packageName) {
      throw unavailable(`no native build for ${platform}-${arch}`);
    }
    try {
      binding = require(packageName);
    } catch (fallback) {
      if (isMissingModule(fallback)) {
        throw unavailable(`${packageName} is not installed for ${platform}-${arch}`);
      }
      throw fallback;
    }
  }

  cached = binding as NativeBinding;
  return cached;
}

function isMissingModule(error: unknown): boolean {
  return (error as { code?: unknown } | null)?.code === "MODULE_NOT_FOUND";
}
