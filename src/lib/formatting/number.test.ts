import { describe, expect, it } from "vitest";

import { formatBytes, formatCompactNumber, formatDuration, formatInteger, formatPercent } from "./number";

describe("formatDuration", () => {
  it("formats sub-minute durations as m:ss", () => {
    expect(formatDuration(47)).toBe("0:47");
    expect(formatDuration(5)).toBe("0:05");
  });

  it("formats durations over a minute", () => {
    expect(formatDuration(90)).toBe("1:30");
    expect(formatDuration(600)).toBe("10:00");
  });

  it("formats durations over an hour as h:mm:ss", () => {
    expect(formatDuration(3661)).toBe("1:01:01");
  });

  it("clamps negative durations to zero", () => {
    expect(formatDuration(-5)).toBe("0:00");
  });
});

describe("formatBytes", () => {
  it("handles zero", () => {
    expect(formatBytes(0)).toBe("0 B");
  });

  it("picks the right unit", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(2048)).toBe("2.0 KB");
    expect(formatBytes(1_500_000)).toBe("1.4 MB");
  });
});

describe("formatCompactNumber", () => {
  it("compacts large numbers", () => {
    expect(formatCompactNumber(1200)).toBe("1.2K");
    expect(formatCompactNumber(4_218_940)).toBe("4.2M");
  });
});

describe("formatInteger", () => {
  it("groups thousands", () => {
    expect(formatInteger(1234567)).toBe("1,234,567");
  });
});

describe("formatPercent", () => {
  it("appends a percent sign with one decimal by default", () => {
    expect(formatPercent(12.4)).toBe("12.4%");
  });

  it("respects a custom fraction digit count", () => {
    expect(formatPercent(12.456, 2)).toBe("12.46%");
  });
});
