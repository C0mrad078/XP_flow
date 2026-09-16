use serde::{Deserialize, Serialize};

use super::capability::Capability;
use super::connection_health::{derive_connection_health, ConnectionHealth};
use super::platform_account::PlatformAccount;
use super::video_status::{AvailabilityStatus, ValidationStatus};

/// Why a publication is not actually ready to go out, computed fresh from
/// existing state (section 74/75) — never mutated into `PublicationStatus`,
/// which stays exactly the state machine Phase 3 built. This is what
/// Phase 5's "Due Publication -> Readiness Check -> PlatformConnector ->
/// Upload" pipeline will consume without needing any redesign here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessIssue {
    AccountNotConnected,
    AccountReauthRequired,
    PublishPermissionMissing,
    VideoMissing,
    VideoInvalid,
    /// Section 22/29 — provider-specific validation of the rendered
    /// metadata failed (title too long, unsupported option value, etc).
    MetadataInvalid,
    /// Section 31/32 — a provider requires proof of explicit user
    /// approval for this exact rendered metadata, and none exists yet
    /// (or the metadata changed since the last approval — section 33).
    ConsentRequired,
    /// Section 105/107-109 — the provider's own developer-app review
    /// status blocks this operation (e.g. an unaudited TikTok app
    /// restricted to private-only posts). Distinct from
    /// `PublishPermissionMissing`: the *account* granted the scope, but
    /// the *application* itself hasn't cleared the provider's own review.
    PlatformNotApproved,
    /// Section 78/129 — the account is inside a provider-supplied
    /// `Retry-After` window recorded in `provider_rate_state`.
    RateLimited,
    /// A locked publication is deliberately excluded from any automatic
    /// action (section 82/113's existing lock semantics) — surfaced here
    /// too so the Publishing Engine's own claim check has one place to
    /// look, consistent with the scheduler's.
    PublicationLocked,
    /// Section 3/66 — a publication that already has a confirmed remote
    /// result must never be treated as ready to publish again; a repost
    /// is a distinct, explicit user action, not implicit readiness.
    AlreadyPublished,
}

/// Inputs are already-loaded, optional references — the caller
/// (`application::publishing_readiness_service`) is responsible for the
/// actual repository lookups; this function stays pure and trivially
/// testable. Every field defaults to "not applicable/not checked" so
/// callers that don't yet have Phase 5's richer state (there are none
/// left after section 19's fix, but the default keeps this struct's
/// construction ergonomic in tests) never accidentally fail a check they
/// didn't actually run.
#[derive(Default)]
pub struct ReadinessInputs<'a> {
    pub platform_account: Option<&'a PlatformAccount>,
    pub video_availability: Option<AvailabilityStatus>,
    pub video_validation: Option<ValidationStatus>,
    pub locked: bool,
    pub already_published: bool,
    /// `None` = this provider doesn't require metadata validation input
    /// here (shouldn't normally happen once wired — see the Publishing
    /// Engine, which always supplies a real result); `Some(false)` = a
    /// `PlatformPublisher::validate_metadata` call failed.
    pub metadata_valid: Option<bool>,
    /// `None` = this provider doesn't require express consent (YouTube,
    /// Kwai); `Some(false)` = required and missing/stale (section 33).
    pub consent_ok: Option<bool>,
    /// `None` = not applicable; `Some(false)` = this provider application
    /// hasn't cleared the provider's own review for this operation.
    pub platform_approved: Option<bool>,
    pub rate_limited_until: Option<chrono::DateTime<chrono::Utc>>,
}

pub fn compute_readiness(inputs: &ReadinessInputs) -> Vec<ReadinessIssue> {
    let mut issues = Vec::new();

    if inputs.already_published {
        issues.push(ReadinessIssue::AlreadyPublished);
    }
    if inputs.locked {
        issues.push(ReadinessIssue::PublicationLocked);
    }
    if inputs.metadata_valid == Some(false) {
        issues.push(ReadinessIssue::MetadataInvalid);
    }
    if inputs.consent_ok == Some(false) {
        issues.push(ReadinessIssue::ConsentRequired);
    }
    if inputs.platform_approved == Some(false) {
        issues.push(ReadinessIssue::PlatformNotApproved);
    }
    if inputs
        .rate_limited_until
        .is_some_and(|until| until > chrono::Utc::now())
    {
        issues.push(ReadinessIssue::RateLimited);
    }

    match inputs.platform_account {
        None => issues.push(ReadinessIssue::AccountNotConnected),
        Some(account) => {
            match derive_connection_health(account, chrono::Utc::now()) {
                ConnectionHealth::Disconnected => issues.push(ReadinessIssue::AccountNotConnected),
                ConnectionHealth::RefreshRequired => {
                    issues.push(ReadinessIssue::AccountReauthRequired)
                }
                _ => {}
            }
            if !account.has_capability(Capability::UploadVideo) {
                issues.push(ReadinessIssue::PublishPermissionMissing);
            }
        }
    }

    match inputs.video_availability {
        Some(AvailabilityStatus::Available) | None => {}
        Some(_) => issues.push(ReadinessIssue::VideoMissing),
    }

    match inputs.video_validation {
        Some(ValidationStatus::Valid) | None => {}
        Some(_) => issues.push(ReadinessIssue::VideoInvalid),
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::platform::Platform;
    use crate::domain::platform_account::PlatformAccountStatus;
    use uuid::Uuid;

    fn connected_account_without_upload() -> PlatformAccount {
        let mut account = PlatformAccount::new(Uuid::new_v4(), Uuid::new_v4(), Platform::YouTube);
        account.status = PlatformAccountStatus::Connected;
        account.capabilities = vec![Capability::ReadProfile];
        account
    }

    #[test]
    fn no_account_is_not_ready() {
        let issues = compute_readiness(&ReadinessInputs {
            platform_account: None,
            video_availability: Some(AvailabilityStatus::Available),
            video_validation: Some(ValidationStatus::Valid),
            ..Default::default()
        });
        assert_eq!(issues, vec![ReadinessIssue::AccountNotConnected]);
    }

    #[test]
    fn connected_account_without_upload_capability_is_flagged() {
        let account = connected_account_without_upload();
        let issues = compute_readiness(&ReadinessInputs {
            platform_account: Some(&account),
            video_availability: Some(AvailabilityStatus::Available),
            video_validation: Some(ValidationStatus::Valid),
            ..Default::default()
        });
        assert_eq!(issues, vec![ReadinessIssue::PublishPermissionMissing]);
    }

    #[test]
    fn reauth_required_account_is_flagged_distinctly_from_not_connected() {
        let mut account = connected_account_without_upload();
        account.status = PlatformAccountStatus::ReauthRequired;
        account.capabilities = vec![Capability::ReadProfile, Capability::UploadVideo];
        let issues = compute_readiness(&ReadinessInputs {
            platform_account: Some(&account),
            video_availability: Some(AvailabilityStatus::Available),
            video_validation: Some(ValidationStatus::Valid),
            ..Default::default()
        });
        assert_eq!(issues, vec![ReadinessIssue::AccountReauthRequired]);
    }

    #[test]
    fn missing_video_is_flagged_independently_of_account_state() {
        let mut account = connected_account_without_upload();
        account.capabilities = vec![Capability::ReadProfile, Capability::UploadVideo];
        let issues = compute_readiness(&ReadinessInputs {
            platform_account: Some(&account),
            video_availability: Some(AvailabilityStatus::Missing),
            video_validation: Some(ValidationStatus::Valid),
            ..Default::default()
        });
        assert_eq!(issues, vec![ReadinessIssue::VideoMissing]);
    }

    #[test]
    fn fully_ready_publication_has_no_issues() {
        let mut account = connected_account_without_upload();
        account.capabilities = vec![Capability::ReadProfile, Capability::UploadVideo];
        let issues = compute_readiness(&ReadinessInputs {
            platform_account: Some(&account),
            video_availability: Some(AvailabilityStatus::Available),
            video_validation: Some(ValidationStatus::Valid),
            ..Default::default()
        });
        assert!(issues.is_empty());
    }

    fn ready_account() -> PlatformAccount {
        let mut account = connected_account_without_upload();
        account.capabilities = vec![Capability::ReadProfile, Capability::UploadVideo];
        account
    }

    #[test]
    fn a_locked_publication_is_not_ready_regardless_of_everything_else() {
        let account = ready_account();
        let issues = compute_readiness(&ReadinessInputs {
            platform_account: Some(&account),
            video_availability: Some(AvailabilityStatus::Available),
            video_validation: Some(ValidationStatus::Valid),
            locked: true,
            ..Default::default()
        });
        assert_eq!(issues, vec![ReadinessIssue::PublicationLocked]);
    }

    #[test]
    fn an_already_published_publication_is_never_ready_again() {
        let account = ready_account();
        let issues = compute_readiness(&ReadinessInputs {
            platform_account: Some(&account),
            video_availability: Some(AvailabilityStatus::Available),
            video_validation: Some(ValidationStatus::Valid),
            already_published: true,
            ..Default::default()
        });
        assert_eq!(issues, vec![ReadinessIssue::AlreadyPublished]);
    }

    #[test]
    fn missing_consent_and_invalid_metadata_are_flagged_independently() {
        let account = ready_account();
        let issues = compute_readiness(&ReadinessInputs {
            platform_account: Some(&account),
            video_availability: Some(AvailabilityStatus::Available),
            video_validation: Some(ValidationStatus::Valid),
            metadata_valid: Some(false),
            consent_ok: Some(false),
            ..Default::default()
        });
        assert!(issues.contains(&ReadinessIssue::MetadataInvalid));
        assert!(issues.contains(&ReadinessIssue::ConsentRequired));
        assert_eq!(issues.len(), 2);
    }

    #[test]
    fn a_future_rate_limit_window_is_flagged_but_a_past_one_is_not() {
        let account = ready_account();
        let issues = compute_readiness(&ReadinessInputs {
            platform_account: Some(&account),
            video_availability: Some(AvailabilityStatus::Available),
            video_validation: Some(ValidationStatus::Valid),
            rate_limited_until: Some(chrono::Utc::now() + chrono::Duration::minutes(5)),
            ..Default::default()
        });
        assert_eq!(issues, vec![ReadinessIssue::RateLimited]);

        let issues = compute_readiness(&ReadinessInputs {
            platform_account: Some(&account),
            video_availability: Some(AvailabilityStatus::Available),
            video_validation: Some(ValidationStatus::Valid),
            rate_limited_until: Some(chrono::Utc::now() - chrono::Duration::minutes(5)),
            ..Default::default()
        });
        assert!(issues.is_empty());
    }
}
