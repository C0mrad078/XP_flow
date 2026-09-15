/**
 * All timestamps arrive from the backend as UTC ISO-8601 strings (section
 * 10: "Prefer UTC internally. Convert to local timezone only for
 * display."). Every formatter here does that conversion — nothing else in
 * the app should call `Date` formatting directly on a raw backend
 * timestamp.
 */

const timeFormatter = new Intl.DateTimeFormat(undefined, {
  hour: "numeric",
  minute: "2-digit",
});

const dateTimeFormatter = new Intl.DateTimeFormat(undefined, {
  month: "short",
  day: "numeric",
  hour: "numeric",
  minute: "2-digit",
});

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  month: "short",
  day: "numeric",
  year: "numeric",
});

export function formatTime(isoDateTime: string): string {
  return timeFormatter.format(new Date(isoDateTime));
}

export function formatDateTime(isoDateTime: string): string {
  return dateTimeFormatter.format(new Date(isoDateTime));
}

export function formatDate(isoDateTime: string): string {
  return dateFormatter.format(new Date(isoDateTime));
}

/** Queue/Calendar formatters that respect the *workspace's* configured
 * IANA timezone (section 19) rather than the OS/browser's — the two are
 * not guaranteed to match, and the backend is explicit that scheduling
 * must never rely blindly on the local machine's zone. */
export function formatTimeInZone(isoDateTime: string, timezone: string): string {
  return new Intl.DateTimeFormat(undefined, {
    hour: "numeric",
    minute: "2-digit",
    timeZone: timezone,
  }).format(new Date(isoDateTime));
}

export function formatDateTimeInZone(isoDateTime: string, timezone: string): string {
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    hour: "numeric",
    minute: "2-digit",
    timeZone: timezone,
  }).format(new Date(isoDateTime));
}

export function formatDateInZone(isoDateTime: string, timezone: string): string {
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
    timeZone: timezone,
  }).format(new Date(isoDateTime));
}

/** "YYYY-MM-DD" in `timezone` — a stable grouping/comparison key, not for display. */
export function localDateKeyInZone(isoDateTime: string, timezone: string): string {
  return new Intl.DateTimeFormat("en-CA", { timeZone: timezone }).format(new Date(isoDateTime));
}

export function formatRelativeTime(isoDateTime: string, now: Date = new Date()): string {
  const date = new Date(isoDateTime);
  const diffMs = date.getTime() - now.getTime();
  const diffSeconds = Math.round(diffMs / 1000);
  const absSeconds = Math.abs(diffSeconds);

  const rtf = new Intl.RelativeTimeFormat(undefined, { numeric: "auto" });

  if (absSeconds < 60) return rtf.format(diffSeconds, "second");
  const diffMinutes = Math.round(diffSeconds / 60);
  if (Math.abs(diffMinutes) < 60) return rtf.format(diffMinutes, "minute");
  const diffHours = Math.round(diffMinutes / 60);
  if (Math.abs(diffHours) < 24) return rtf.format(diffHours, "hour");
  const diffDays = Math.round(diffHours / 24);
  if (Math.abs(diffDays) < 30) return rtf.format(diffDays, "day");
  const diffMonths = Math.round(diffDays / 30);
  return rtf.format(diffMonths, "month");
}
