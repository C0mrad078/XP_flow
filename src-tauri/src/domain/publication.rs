use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::errors::{DomainError, DomainResult};
use super::platform::Platform;
use super::video_status::VideoPriority;

/// The lifecycle of a single [`Publication`].
///
/// A publication moves forward through the "happy path" states
/// (`Imported -> Validating -> Ready -> Queued -> Scheduled -> Uploading ->
/// Processing -> Published`) and can drop into an operational/failure state
/// at almost any point. Transitions are validated centrally by
/// [`Publication::transition`] so no code path can move a publication into
/// an impossible state by mutating a raw string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationStatus {
    // Happy path
    Imported,
    Validating,
    Ready,
    Queued,
    Scheduled,
    Uploading,
    Processing,
    Published,

    // Failure / operational states
    Failed,
    RetryWait,
    AuthRequired,
    RateLimited,
    Blocked,
    Paused,
    Cancelled,
    Archived,
    Duplicate,
}

impl PublicationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            PublicationStatus::Imported => "imported",
            PublicationStatus::Validating => "validating",
            PublicationStatus::Ready => "ready",
            PublicationStatus::Queued => "queued",
            PublicationStatus::Scheduled => "scheduled",
            PublicationStatus::Uploading => "uploading",
            PublicationStatus::Processing => "processing",
            PublicationStatus::Published => "published",
            PublicationStatus::Failed => "failed",
            PublicationStatus::RetryWait => "retry_wait",
            PublicationStatus::AuthRequired => "auth_required",
            PublicationStatus::RateLimited => "rate_limited",
            PublicationStatus::Blocked => "blocked",
            PublicationStatus::Paused => "paused",
            PublicationStatus::Cancelled => "cancelled",
            PublicationStatus::Archived => "archived",
            PublicationStatus::Duplicate => "duplicate",
        }
    }

    /// Terminal states never have an outgoing transition, `Archived` most of
    /// all — it is the deliberate "nothing more will ever happen" end state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, PublicationStatus::Archived)
    }

    /// The set of states this status is allowed to move to directly.
    pub fn allowed_next(&self) -> &'static [PublicationStatus] {
        use PublicationStatus::*;
        match self {
            Imported => &[Validating, Cancelled, Duplicate],
            Validating => &[Ready, Failed, Duplicate],
            Ready => &[Queued, Cancelled, Archived],
            Queued => &[Scheduled, Paused, Cancelled],
            Scheduled => &[Uploading, Queued, Paused, Cancelled],
            Uploading => &[Processing, Failed, AuthRequired, RateLimited, Blocked],
            Processing => &[Published, Failed],
            Published => &[Archived],
            Failed => &[RetryWait, Cancelled, Archived],
            RetryWait => &[Queued, Uploading, Cancelled],
            AuthRequired => &[Queued, Cancelled],
            RateLimited => &[RetryWait, Cancelled],
            Blocked => &[Cancelled, Archived],
            Paused => &[Queued, Scheduled, Cancelled],
            Cancelled => &[Archived],
            Archived => &[],
            Duplicate => &[Archived, Cancelled],
        }
    }

    pub fn can_transition_to(&self, next: PublicationStatus) -> bool {
        self.allowed_next().contains(&next)
    }
}

impl std::fmt::Display for PublicationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for PublicationStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use PublicationStatus::*;
        Ok(match s {
            "imported" => Imported,
            "validating" => Validating,
            "ready" => Ready,
            "queued" => Queued,
            "scheduled" => Scheduled,
            "uploading" => Uploading,
            "processing" => Processing,
            "published" => Published,
            "failed" => Failed,
            "retry_wait" => RetryWait,
            "auth_required" => AuthRequired,
            "rate_limited" => RateLimited,
            "blocked" => Blocked,
            "paused" => Paused,
            "cancelled" => Cancelled,
            "archived" => Archived,
            "duplicate" => Duplicate,
            other => return Err(format!("unknown publication status: {other}")),
        })
    }
}

/// Per-field metadata resolution controls (Phase 5.1 section 5/40) layered
/// on top of a publication's own raw `title`/`description`/`hashtags` —
/// the lowest-precedence "Video / Publication Raw Fields" rung of the
/// ladder documented in `docs/metadata-templates.md`. Every field defaults
/// to `None`, meaning "resolve automatically through the channel/workspace
/// template precedence chain" — nothing here changes behavior for a
/// publication that has never touched the metadata template system.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetadataOverrides {
    /// Literal text for this one publication (still rendered through
    /// `{variables}`), taking precedence over every template — the
    /// "Publication Override" rung.
    pub title_override: Option<String>,
    pub description_override: Option<String>,
    /// Literal hashtag list for this one publication.
    pub hashtags_override: Option<Vec<String>>,
    /// Pins resolution to one specific template rather than letting the
    /// channel/workspace precedence chain pick ("Select specific
    /// template" in the editor). Cleared automatically (`ON DELETE SET
    /// NULL`) if that template is ever deleted — resolution then falls
    /// through to the next rung, never a dangling reference.
    pub title_template_id: Option<Uuid>,
    pub description_template_id: Option<Uuid>,
    pub hashtag_set_id: Option<Uuid>,
    /// Provider-specific option overrides (YouTube privacy/category,
    /// TikTok privacy_level/duet/stitch/comment toggles, Kwai cover/
    /// caption) as a JSON object — opaque here, interpreted only by each
    /// `PlatformPublisher`, same discipline as
    /// `RenderedMetadata::provider_options`.
    pub provider_options_override: Option<serde_json::Value>,
}

/// A single platform-specific publication of a [`super::video::Video`].
///
/// A video may fan out into several publications — one per platform — and
/// each tracks its own title/description/schedule/status independently.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Publication {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub video_id: Uuid,
    pub channel_id: Uuid,
    pub platform_account_id: Option<Uuid>,
    pub platform: Platform,
    pub status: PublicationStatus,
    pub title: String,
    pub description: Option<String>,
    pub hashtags: Vec<String>,
    /// Operational priority for scheduling/ordering (section 12/13). Seeded
    /// from the source video's priority at creation time, but independently
    /// mutable afterwards — a publication may need to be bumped or lowered
    /// without touching the underlying video.
    pub priority: VideoPriority,
    /// When true, the auto-scheduler and "Rebuild Schedule" must never move
    /// `scheduled_at` for this publication (section 82/113).
    pub locked: bool,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub remote_id: Option<String>,
    pub retry_count: i32,
    pub last_error: Option<String>,
    /// The stable `PublishError::code()` behind `last_error`, when it
    /// came from a typed error — see `ExecutionStateUpdate::last_error_code`.
    pub last_error_code: Option<String>,
    /// Section 67 — the identity a retry reuses across attempts; a
    /// deliberate user-initiated repost gets a *new* execution key
    /// instead (section 3), so "same execution key" is exactly XP FLOW's
    /// definition of "the same logical remote write."
    pub execution_key: Option<Uuid>,
    /// The prior publication deliberately reposted to create this one.
    /// One direct child per source makes repeated clicks idempotent; another
    /// intentional repost can be made from that child.
    pub repost_of_publication_id: Option<Uuid>,
    pub reconciliation_result: Option<String>,
    pub reconciled_at: Option<DateTime<Utc>>,
    /// Section 13 — proves current ownership of an active claim; cleared
    /// whenever the publication leaves `Uploading`/`Processing`.
    pub claim_token: Option<String>,
    /// Section 13/88 — a claim past this instant is considered abandoned
    /// (the process that made it may have crashed) and eligible for
    /// crash-recovery reconciliation.
    pub lease_expires_at: Option<DateTime<Utc>>,
    /// Section 103 — the exact metadata sent for the current/most recent
    /// attempt, frozen the moment execution starts so a later template
    /// edit can never retroactively change what an in-flight or already-
    /// executed attempt claims it sent.
    pub rendered_metadata: Option<crate::domain::publishing::RenderedMetadata>,
    /// Phase 5.1 — per-field metadata resolution controls; see
    /// [`MetadataOverrides`].
    pub metadata_overrides: MetadataOverrides,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Publication {
    pub fn new(
        workspace_id: Uuid,
        video_id: Uuid,
        channel_id: Uuid,
        platform: Platform,
        title: impl Into<String>,
        priority: VideoPriority,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            video_id,
            channel_id,
            platform_account_id: None,
            platform,
            status: PublicationStatus::Imported,
            title: title.into(),
            description: None,
            hashtags: Vec::new(),
            priority,
            locked: false,
            scheduled_at: None,
            published_at: None,
            remote_id: None,
            retry_count: 0,
            last_error: None,
            last_error_code: None,
            execution_key: None,
            repost_of_publication_id: None,
            reconciliation_result: None,
            reconciled_at: None,
            claim_token: None,
            lease_expires_at: None,
            rendered_metadata: None,
            metadata_overrides: MetadataOverrides::default(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Whether this publication is currently overdue: still `Scheduled` but
    /// its `scheduled_at` has already passed. This is a *derived* label
    /// (section 78/81) — never persisted, never mutated to `Failed` just
    /// because Phase 3 has no real uploader yet.
    pub fn is_overdue(&self, now: DateTime<Utc>) -> bool {
        self.status == PublicationStatus::Scheduled && self.scheduled_at.is_some_and(|at| at < now)
    }

    /// Attempts to move this publication to `next`, validating the
    /// transition against [`PublicationStatus::allowed_next`]. Returns
    /// [`DomainError::InvalidTransition`] rather than silently accepting an
    /// impossible state change.
    pub fn transition(&mut self, next: PublicationStatus) -> DomainResult<()> {
        if !self.status.can_transition_to(next) {
            return Err(DomainError::InvalidTransition {
                entity: "Publication",
                from: self.status.to_string(),
                to: next.to_string(),
            });
        }

        if next == PublicationStatus::Published {
            self.published_at = Some(Utc::now());
        }

        self.status = next;
        self.updated_at = Utc::now();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_publication() -> Publication {
        Publication::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Uuid::new_v4(),
            Platform::YouTube,
            "Sample title",
            VideoPriority::Normal,
        )
    }

    #[test]
    fn new_publication_starts_imported() {
        let publication = sample_publication();
        assert_eq!(publication.status, PublicationStatus::Imported);
    }

    #[test]
    fn happy_path_transitions_succeed() {
        let mut publication = sample_publication();
        let happy_path = [
            PublicationStatus::Validating,
            PublicationStatus::Ready,
            PublicationStatus::Queued,
            PublicationStatus::Scheduled,
            PublicationStatus::Uploading,
            PublicationStatus::Processing,
            PublicationStatus::Published,
            PublicationStatus::Archived,
        ];

        for next in happy_path {
            publication
                .transition(next)
                .unwrap_or_else(|e| panic!("expected transition to {next:?} to succeed, got {e}"));
        }

        assert_eq!(publication.status, PublicationStatus::Archived);
        assert!(publication.published_at.is_some());
    }

    #[test]
    fn cannot_skip_states() {
        let mut publication = sample_publication();
        let err = publication
            .transition(PublicationStatus::Published)
            .expect_err("Imported -> Published must be rejected");

        match err {
            DomainError::InvalidTransition { from, to, .. } => {
                assert_eq!(from, "imported");
                assert_eq!(to, "published");
            }
            other => panic!("expected InvalidTransition, got {other:?}"),
        }
    }

    #[test]
    fn cannot_transition_out_of_archived() {
        let mut publication = sample_publication();
        for next in [
            PublicationStatus::Validating,
            PublicationStatus::Ready,
            PublicationStatus::Queued,
            PublicationStatus::Scheduled,
            PublicationStatus::Uploading,
            PublicationStatus::Processing,
            PublicationStatus::Published,
            PublicationStatus::Archived,
        ] {
            publication.transition(next).unwrap();
        }

        assert!(publication.status.is_terminal());
        assert!(publication.transition(PublicationStatus::Queued).is_err());
    }

    #[test]
    fn failure_and_retry_loop_is_valid() {
        let mut publication = sample_publication();
        for next in [
            PublicationStatus::Validating,
            PublicationStatus::Ready,
            PublicationStatus::Queued,
            PublicationStatus::Scheduled,
            PublicationStatus::Uploading,
        ] {
            publication.transition(next).unwrap();
        }

        publication.transition(PublicationStatus::Failed).unwrap();
        publication
            .transition(PublicationStatus::RetryWait)
            .unwrap();
        publication.transition(PublicationStatus::Queued).unwrap();
        assert_eq!(publication.status, PublicationStatus::Queued);
    }

    #[test]
    fn overdue_is_derived_from_status_and_scheduled_at() {
        let mut publication = sample_publication();
        let now = Utc::now();

        assert!(!publication.is_overdue(now), "not scheduled yet");

        for next in [
            PublicationStatus::Validating,
            PublicationStatus::Ready,
            PublicationStatus::Queued,
        ] {
            publication.transition(next).unwrap();
        }
        publication.scheduled_at = Some(now - chrono::Duration::hours(1));
        publication
            .transition(PublicationStatus::Scheduled)
            .unwrap();

        assert!(publication.is_overdue(now), "past due and still scheduled");

        publication
            .transition(PublicationStatus::Uploading)
            .unwrap();
        assert!(
            !publication.is_overdue(now),
            "no longer just Scheduled, so no longer counted as overdue"
        );
    }

    #[test]
    fn status_round_trips_through_string() {
        for status in [
            PublicationStatus::Imported,
            PublicationStatus::Validating,
            PublicationStatus::Ready,
            PublicationStatus::Queued,
            PublicationStatus::Scheduled,
            PublicationStatus::Uploading,
            PublicationStatus::Processing,
            PublicationStatus::Published,
            PublicationStatus::Failed,
            PublicationStatus::RetryWait,
            PublicationStatus::AuthRequired,
            PublicationStatus::RateLimited,
            PublicationStatus::Blocked,
            PublicationStatus::Paused,
            PublicationStatus::Cancelled,
            PublicationStatus::Archived,
            PublicationStatus::Duplicate,
        ] {
            let parsed: PublicationStatus = status.as_str().parse().unwrap();
            assert_eq!(parsed, status);
        }
    }
}
