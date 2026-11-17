import { describe, expect, it } from "vitest";
import { platformPackageName } from "../src/loader.ts";

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
