/**
 * Pure calendar-grid date math. This is deliberately *not* timezone-aware
 * — a calendar "day" here is just a `Date` used to derive a "YYYY-MM-DD"
 * key, and grid layout (which dates belong in which cell) never depends on
 * a timezone. The only place timezone actually matters — bucketing a
 * publication's UTC `scheduled_at` onto the correct local day — already
 * happened backend-side (`CalendarPublication.local_date`), so nothing
 * here needs `Intl`/`chrono-tz` equivalents.
 */

export function toDateKey(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, "0");
  const d = String(date.getDate()).padStart(2, "0");
  return `${y}-${m}-${d}`;
}

function addDays(date: Date, days: number): Date {
  const next = new Date(date);
  next.setDate(next.getDate() + days);
  return next;
}

/** Monday-start week containing `date`. */
export function startOfWeek(date: Date): Date {
  const day = date.getDay(); // 0 = Sunday .. 6 = Saturday
  const diff = day === 0 ? -6 : 1 - day;
  const start = new Date(date);
  start.setDate(start.getDate() + diff);
  start.setHours(0, 0, 0, 0);
  return start;
}

/** The 7 dates of the Monday-start week containing `date`. */
export function weekDates(date: Date): Date[] {
  const start = startOfWeek(date);
  return Array.from({ length: 7 }, (_, i) => addDays(start, i));
}

/** The full 6-week (42-day) grid for the month containing `date`,
 * including leading/trailing days from adjacent months so every week row
 * is complete. */
export function monthGridDates(date: Date): Date[] {
  const firstOfMonth = new Date(date.getFullYear(), date.getMonth(), 1);
  const gridStart = startOfWeek(firstOfMonth);
  return Array.from({ length: 42 }, (_, i) => addDays(gridStart, i));
}

export function isSameMonth(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth();
}

export function isSameDay(a: Date, b: Date): boolean {
  return toDateKey(a) === toDateKey(b);
}
