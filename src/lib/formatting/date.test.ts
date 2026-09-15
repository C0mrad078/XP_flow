import { describe, expect, it } from "vitest";

import { formatTimeInZone, localDateKeyInZone } from "./date";

describe("formatTimeInZone", () => {
  it("formats a UTC instant in the given IANA timezone, not the host's", () => {
    // 2024-06-15T14:00:00Z is 10:00 in America/New_York (EDT, UTC-4).
    expect(formatTimeInZone("2024-06-15T14:00:00Z", "America/New_York")).toMatch(/^10:00/);
  });

  it("differs across timezones for the same instant", () => {
    const utc = formatTimeInZone("2024-06-15T14:00:00Z", "UTC");
    const tokyo = formatTimeInZone("2024-06-15T14:00:00Z", "Asia/Tokyo");
    expect(utc).not.toBe(tokyo);
  });
});

describe("localDateKeyInZone", () => {
  it("can land on the previous calendar day in a timezone west of UTC", () => {
    // 2024-06-15T02:00:00Z is still 2024-06-14 in America/Los_Angeles (PDT, UTC-7).
    expect(localDateKeyInZone("2024-06-15T02:00:00Z", "America/Los_Angeles")).toBe("2024-06-14");
    expect(localDateKeyInZone("2024-06-15T02:00:00Z", "UTC")).toBe("2024-06-15");
  });
});
