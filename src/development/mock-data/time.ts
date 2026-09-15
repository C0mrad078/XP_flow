/** Relative-time helpers so mock data always looks "live" instead of
 * hardcoding stale dates. Mock-only — never imported outside development/. */
export function hoursFromNow(hours: number): string {
  return new Date(Date.now() + hours * 60 * 60 * 1000).toISOString();
}

export function daysFromNow(days: number): string {
  return hoursFromNow(days * 24);
}
