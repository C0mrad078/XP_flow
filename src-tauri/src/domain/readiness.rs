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
}

/// Inputs are already-loaded, optional references — the caller
/// (`application::publication_service`) is responsible for the actual
/// repository lookups; this function stays pure and trivially testable.
pub struct ReadinessInputs<'a> {
    pub platform_account: Option<&'a PlatformAccount>,
    pub video_availability: Option<AvailabilityStatus>,
    pub video_validation: Option<ValidationStatus>,
}

pub fn compute_readiness(inputs: &ReadinessInputs) -> Vec<ReadinessIssue> {
    let mut issues = Vec::new();

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
        });
        assert!(issues.is_empty());
    }
}
