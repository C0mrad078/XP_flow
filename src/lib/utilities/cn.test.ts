import { describe, expect, it } from "vitest";

import { cn } from "./cn";

describe("cn", () => {
  it("joins truthy class names", () => {
    const skipped = false;
    expect(cn("a", "b", skipped && "c", undefined, "d")).toBe("a b d");
  });

  it("resolves conflicting Tailwind utility classes, last one winning", () => {
    expect(cn("px-2 py-1", "px-4")).toBe("py-1 px-4");
  });

  it("supports conditional object syntax", () => {
    expect(cn("base", { active: true, disabled: false })).toBe("base active");
  });
});
