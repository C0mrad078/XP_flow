import { afterEach, describe, expect, it, vi } from "vitest";

import { resolveTheme } from "./theme";

describe("resolveTheme", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("passes dark and light through unchanged", () => {
    expect(resolveTheme("dark")).toBe("dark");
    expect(resolveTheme("light")).toBe("light");
  });

  it("resolves system to light when the OS prefers light", () => {
    vi.stubGlobal("matchMedia", (query: string) => ({ matches: query.includes("light") }) as MediaQueryList);
    expect(resolveTheme("system")).toBe("light");
  });

  it("resolves system to dark when the OS does not prefer light", () => {
    vi.stubGlobal("matchMedia", () => ({ matches: false }) as MediaQueryList);
    expect(resolveTheme("system")).toBe("dark");
  });
});
