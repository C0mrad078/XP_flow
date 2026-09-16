use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    #[default]
    Dark,
    Light,
    System,
}

impl ThemePreference {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemePreference::Dark => "dark",
            ThemePreference::Light => "light",
            ThemePreference::System => "system",
        }
    }
}

impl std::str::FromStr for ThemePreference {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dark" => Ok(ThemePreference::Dark),
            "light" => Ok(ThemePreference::Light),
            "system" => Ok(ThemePreference::System),
            other => Err(format!("unknown theme preference: {other}")),
        }
    }
}

/// How a `Scheduled` publication that's overdue *beyond* its configured
/// grace period is handled (Phase 5.1 section 55/57) — distinct from
/// simply being late, which the periodic scan still catches and
/// publishes normally within the grace window regardless of which
/// policy is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MissedSchedulePolicy {
    /// Still publish as long as it's within the grace period; beyond
    /// that, leave it `Scheduled` and surface it as needing review
    /// rather than either force-publishing very stale content or
    /// silently discarding it.
    #[default]
    PublishWithinGrace,
    /// Never auto-claim an overdue publication — every miss needs a
    /// human decision, even a one-minute miss.
    NeedsReview,
    /// An overdue-beyond-grace publication is moved to `Cancelled` (the
    /// state machine has no `Failed` transition directly from
    /// `Scheduled`, so this reuses the existing terminal state — the
    /// distinguishing signal is `last_error` and the activity log entry,
    /// not a separate status) with a clear reason recorded.
    Skip,
}

impl MissedSchedulePolicy {
    pub fn as_str(&self) -> &'static str {
        match self {
            MissedSchedulePolicy::PublishWithinGrace => "publish_within_grace",
            MissedSchedulePolicy::NeedsReview => "needs_review",
            MissedSchedulePolicy::Skip => "skip",
        }
    }
}

impl std::fmt::Display for MissedSchedulePolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

fn default_max_concurrent_uploads() -> u32 {
    2
}

fn default_grace_period_minutes() -> u32 {
    60
}

fn default_publishing_enabled() -> bool {
    true
}

/// Section 55-58: the workspace-wide controls for the publishing engine.
/// Every field defaults such that an already-persisted `app_settings`
/// row deserializes exactly as if publishing had always been on and
/// unpaused — this is additive, not a behavior change for an existing
/// installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishingSettings {
    #[serde(default = "default_publishing_enabled")]
    pub enabled: bool,
    /// A hard stop distinct from `enabled` (section 58): `enabled` is
    /// the workspace's own long-term choice to use real publishing at
    /// all; `paused` is the quick, reversible "stop everything right
    /// now" toggle surfaced on the Dashboard.
    #[serde(default)]
    pub paused: bool,
    /// Sized into `JobRunner::spawn_periodic_publish_scan`'s upload
    /// semaphore at startup (section 56) — a change here takes effect on
    /// the next app restart, not live; see `docs/publishing-ui.md`.
    #[serde(default = "default_max_concurrent_uploads")]
    pub max_concurrent_uploads: u32,
    #[serde(default)]
    pub missed_schedule_policy: MissedSchedulePolicy,
    #[serde(default = "default_grace_period_minutes")]
    pub missed_schedule_grace_period_minutes: u32,
}

impl Default for PublishingSettings {
    fn default() -> Self {
        Self {
            enabled: default_publishing_enabled(),
            paused: false,
            max_concurrent_uploads: default_max_concurrent_uploads(),
            missed_schedule_policy: MissedSchedulePolicy::default(),
            missed_schedule_grace_period_minutes: default_grace_period_minutes(),
        }
    }
}

impl PublishingSettings {
    /// Never allow an absurd value in either direction (section 56) —
    /// zero would silently stall every upload forever, and an
    /// unbounded number risks a real resource/rate-limit problem on a
    /// user's machine.
    pub fn clamp(mut self) -> Self {
        self.max_concurrent_uploads = self.max_concurrent_uploads.clamp(1, 10);
        self.missed_schedule_grace_period_minutes =
            self.missed_schedule_grace_period_minutes.clamp(0, 24 * 60);
        self
    }
}

/// Application-wide settings. Persisted as a single key/value table
/// (`app_settings`) so future settings can be added without a migration for
/// every new field; this struct is the typed view the frontend receives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: ThemePreference,
    pub launch_on_startup: bool,
    #[serde(default)]
    pub publishing: PublishingSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::Dark,
            launch_on_startup: false,
            publishing: PublishingSettings::default(),
        }
    }
}
