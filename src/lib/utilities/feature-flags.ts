/**
 * Simple, static feature flags (section 42). No remote config, no
 * percentage rollouts — just a single source of truth for which
 * not-yet-real features stay disabled in the UI until a later phase
 * implements them for real.
 */
export const featureFlags = {
  analyticsAdvanced: false,
  platformYouTube: false,
  platformTikTok: false,
  platformKwai: false,
  commentsAutomation: false,
  queueKanbanView: false,
  queueCalendarView: false,
} as const;

export type FeatureFlag = keyof typeof featureFlags;

export function isFeatureEnabled(flag: FeatureFlag): boolean {
  return featureFlags[flag];
}
