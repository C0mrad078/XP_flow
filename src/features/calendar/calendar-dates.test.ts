import { describe, expect, it } from "vitest";

import { isSameDay, isSameMonth, monthGridDates, startOfWeek, toDateKey, weekDates } from "./calendar-dates";

describe("toDateKey", () => {
  it("formats as YYYY-MM-DD", () => {
    expect(toDateKey(new Date(2024, 5, 3))).toBe("2024-06-03");
  });

  it("zero-pads single-digit months and days", () => {
    expect(toDateKey(new Date(2024, 0, 1))).toBe("2024-01-01");
  });
});

describe("startOfWeek", () => {
  it("returns the Monday for a mid-week date", () => {
    // 2024-06-13 is a Thursday.
    expect(toDateKey(startOfWeek(new Date(2024, 5, 13)))).toBe("2024-06-10");
  });

  it("returns the same date when already Monday", () => {
    expect(toDateKey(startOfWeek(new Date(2024, 5, 10)))).toBe("2024-06-10");
  });

  it("rolls a Sunday back to the Monday of the same week, not the next one", () => {
    // 2024-06-16 is a Sunday; its Monday is 2024-06-10.
    expect(toDateKey(startOfWeek(new Date(2024, 5, 16)))).toBe("2024-06-10");
  });
});

describe("weekDates", () => {
  it("returns exactly 7 consecutive dates starting Monday", () => {
    const dates = weekDates(new Date(2024, 5, 13));
    expect(dates).toHaveLength(7);
    expect(toDateKey(dates[0]!)).toBe("2024-06-10");
    expect(toDateKey(dates[6]!)).toBe("2024-06-16");
  });
});

describe("monthGridDates", () => {
  it("returns exactly 42 dates (6 full weeks)", () => {
    expect(monthGridDates(new Date(2024, 5, 13))).toHaveLength(42);
  });

  it("always starts on a Monday and ends on a Sunday", () => {
    const dates = monthGridDates(new Date(2024, 1, 15)); // February 2024
    expect(dates[0]!.getDay()).toBe(1); // Monday
    expect(dates[41]!.getDay()).toBe(0); // Sunday
  });

  it("includes every day of the target month", () => {
    const dates = monthGridDates(new Date(2024, 1, 1)); // February 2024 (29 days, leap year)
    const daysInFeb = dates.filter((d) => d.getMonth() === 1);
    expect(daysInFeb).toHaveLength(29);
  });
});

describe("isSameMonth / isSameDay", () => {
  it("isSameMonth ignores the day", () => {
    expect(isSameMonth(new Date(2024, 5, 1), new Date(2024, 5, 30))).toBe(true);
    expect(isSameMonth(new Date(2024, 5, 30), new Date(2024, 6, 1))).toBe(false);
  });

  it("isSameDay ignores the time of day", () => {
    expect(isSameDay(new Date(2024, 5, 1, 0, 0), new Date(2024, 5, 1, 23, 59))).toBe(true);
    expect(isSameDay(new Date(2024, 5, 1, 23, 59), new Date(2024, 5, 2, 0, 0))).toBe(false);
  });
});
