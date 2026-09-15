import { describe, expect, it } from "vitest";

import { formatBitrate, formatDurationMs, formatFps, formatResolution } from "./media";

describe("formatDurationMs", () => {
  it("formats milliseconds as m:ss", () => {
    expect(formatDurationMs(42_000)).toBe("0:42");
    expect(formatDurationMs(90_000)).toBe("1:30");
  });

  it("returns a placeholder for null", () => {
    expect(formatDurationMs(null)).toBe("-");
  });
});

describe("formatResolution", () => {
  it("formats width x height", () => {
    expect(formatResolution(1080, 1920)).toBe("1080 x 1920");
  });

  it("returns a placeholder when either dimension is missing", () => {
    expect(formatResolution(null, 1920)).toBe("-");
    expect(formatResolution(1080, null)).toBe("-");
  });
});

describe("formatFps", () => {
  it("rounds to one decimal place", () => {
    expect(formatFps(29.97)).toBe("30 fps");
    expect(formatFps(23.976)).toBe("24 fps");
    expect(formatFps(30)).toBe("30 fps");
  });

  it("returns a placeholder for null", () => {
    expect(formatFps(null)).toBe("-");
  });
});

describe("formatBitrate", () => {
  it("formats megabit rates", () => {
    expect(formatBitrate(4_000_000)).toBe("4.0 Mbps");
  });

  it("formats kilobit rates below 1 Mbps", () => {
    expect(formatBitrate(128_000)).toBe("128 kbps");
  });

  it("returns a placeholder for null or zero", () => {
    expect(formatBitrate(null)).toBe("-");
    expect(formatBitrate(0)).toBe("-");
  });
});
