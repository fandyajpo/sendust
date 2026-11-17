import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  PLATFORM_PACKAGES,
  platformPackageName,
  resolveNative,
  type NativeBinding,
} from "../src/loader.ts";

type RequireLike = (id: string) => unknown;

/** A require stand-in that answers from a map and misses the way Node does. */
function fakeRequire(modules: Record<string, unknown>): RequireLike {
  return (id) => {
    if (Object.hasOwn(modules, id)) {
      const entry = modules[id];
      if (entry instanceof Error) {
        throw entry;
      }
      return entry;
    }
    throw moduleNotFound(id);
  };
}

function moduleNotFound(id: string): Error {
  const error = new Error(`Cannot find module '${id}'`);
  Object.defineProperty(error, "code", { value: "MODULE_NOT_FOUND" });
  return error;
}

interface TaggedError extends Error {
  code?: string;
  sendustCategory?: string;
}

/** Runs a resolution that must fail and returns the error it threw. */
function thrown(fn: () => unknown): TaggedError {
  try {
    fn();
  } catch (error) {
    return error as TaggedError;
  }
  throw new Error("expected resolution to fail");
}

/** The target every failure case below resolves against. */
const linuxX64 = {
  bundled: "./sendust.node",
  platform: "linux",
  arch: "x64",
  libc: "gnu",
} as const;

describe("platform package mapping", () => {
  it("names the optional package for each supported target", () => {
    expect(platformPackageName("linux", "x64", "gnu")).toBe("@sendust/ts-linux-x64-gnu");
    expect(platformPackageName("linux", "arm64", "gnu")).toBe("@sendust/ts-linux-arm64-gnu");
    expect(platformPackageName("darwin", "arm64")).toBe("@sendust/ts-macos-arm64");
    expect(platformPackageName("darwin", "x64")).toBe("@sendust/ts-macos-x64");
    expect(platformPackageName("win32", "x64")).toBe("@sendust/ts-windows-x64");
  });

  it("returns nothing for platforms without a build", () => {
    expect(platformPackageName("linux", "x64", "musl")).toBeUndefined();
    expect(platformPackageName("win32", "arm64")).toBeUndefined();
    expect(platformPackageName("freebsd", "x64")).toBeUndefined();
  });
});

describe("native resolution failures", () => {
  it("names the platform package that is not installed", () => {
    const error = thrown(() => resolveNative({ ...linuxX64, require: fakeRequire({}) }));

    expect(error.code).toBe("CONFIG_NATIVE_UNAVAILABLE");
    expect(error.sendustCategory).toBe("configuration");
    expect(error.message).toContain("@sendust/ts-linux-x64-gnu is not installed for linux-x64");
  });

  it("reports a wrong-architecture binary by code without leaking its path", () => {
    const dlopen = new Error(
      "/Users/someone/node_modules/@sendust/ts-linux-x64-gnu/sendust.node: invalid ELF header",
    );
    Object.defineProperty(dlopen, "code", { value: "ERR_DLOPEN_FAILED" });

    const error = thrown(() =>
      resolveNative({
        ...linuxX64,
        require: fakeRequire({ "@sendust/ts-linux-x64-gnu": dlopen }),
      }),
    );

    expect(error.code).toBe("CONFIG_NATIVE_UNAVAILABLE");
    expect(error.message).toContain("failed to load (ERR_DLOPEN_FAILED)");
    expect(error.message).not.toContain("/Users/");
  });

  it("falls back to an unnamed failure code", () => {
    const error = thrown(() =>
      resolveNative({
        ...linuxX64,
        require: fakeRequire({ "@sendust/ts-linux-x64-gnu": new Error("boom") }),
      }),
    );

    expect(error.code).toBe("CONFIG_NATIVE_UNAVAILABLE");
    expect(error.message).toContain("failed to load (unknown)");
  });

  it("reports a platform with no published build", () => {
    const error = thrown(() =>
      resolveNative({
        ...linuxX64,
        libc: "musl",
        require: fakeRequire({}),
      }),
    );

    expect(error.code).toBe("CONFIG_NATIVE_UNAVAILABLE");
    expect(error.message).toContain("no native build for linux-x64");
  });

  it("uses the bundled addon without consulting the platform package", () => {
    const binding = {
      Transport: class {},
      nativeVersion: () => "0.1.0",
    } as unknown as NativeBinding;
    const calls: string[] = [];
    const fake = (id: string): unknown => {
      calls.push(id);
      if (id === "./sendust.node") {
        return binding;
      }
      throw moduleNotFound(id);
    };

    expect(resolveNative({ ...linuxX64, require: fake })).toBe(binding);
    expect(calls).toEqual(["./sendust.node"]);
  });

  it("rethrows a bundled load failure untouched", () => {
    const broken = new Error("wasm boundary failed");
    const error = thrown(() =>
      resolveNative({ ...linuxX64, require: fakeRequire({ "./sendust.node": broken }) }),
    );

    expect(error).toBe(broken);
  });
});

describe("package manifest", () => {
  const manifest = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8")) as {
    version: string;
    optionalDependencies?: Record<string, string>;
  };

  it("declares exactly the platform packages the loader can resolve", () => {
    const declared = Object.keys(manifest.optionalDependencies ?? {}).sort();
    const loadable = Object.values(PLATFORM_PACKAGES).sort();

    expect(declared).toEqual(loadable);
  });

  it("pins every platform package to the facade version", () => {
    for (const version of Object.values(manifest.optionalDependencies ?? {})) {
      expect(version).toBe(manifest.version);
    }
  });
});
