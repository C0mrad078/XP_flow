use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::domain::platform::Platform;

/// How the user approved this exact publication/metadata combination
/// (section 32/34) — an auditable trail, not just a boolean.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalSource {
    ManualSchedule,
    AddToQueue,
    BulkApproval,
    PublishNow,
}

impl ApprovalSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApprovalSource::ManualSchedule => "manual_schedule",
            ApprovalSource::AddToQueue => "add_to_queue",
            ApprovalSource::BulkApproval => "bulk_approval",
            ApprovalSource::PublishNow => "publish_now",
        }
    }
}

impl std::fmt::Display for ApprovalSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ApprovalSource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "manual_schedule" => ApprovalSource::ManualSchedule,
            "add_to_queue" => ApprovalSource::AddToQueue,
            "bulk_approval" => ApprovalSource::BulkApproval,
            "publish_now" => ApprovalSource::PublishNow,
            other => return Err(format!("unknown approval source: {other}")),
        })
    }
}

/// Proof that the user explicitly approved *this exact* rendered metadata
/// for a TikTok publication before XP FLOW is allowed to transmit it
/// (section 31/32). Modeled generically (not TikTok-only in the type
/// system) since nothing about the shape is TikTok-specific — only the
/// readiness check that requires it is provider-gated.
///
/// No `invalidated_at` field: a consent row is only ever valid for the
/// exact metadata it was recorded against. `PublicationConsent::covers`
/// is the whole invalidation mechanism (section 33) — if the rendered
/// metadata hash changed since approval, the old row simply stops
/// matching, with no separate flag to remember to flip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationConsent {
    pub id: Uuid,
    pub publication_id: Uuid,
    pub provider: Platform,
    pub approved_at: DateTime<Utc>,
    pub approved_metadata_hash: String,
    pub approval_source: ApprovalSource,
    pub created_at: DateTime<Utc>,
}

impl PublicationConsent {
    pub fn new(
        publication_id: Uuid,
        provider: Platform,
        approved_metadata_hash: String,
        approval_source: ApprovalSource,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            publication_id,
            provider,
            approved_at: now,
            approved_metadata_hash,
            approval_source,
            created_at: now,
        }
    }

    /// Whether this consent record still covers `current_metadata_hash`.
    pub fn covers(&self, current_metadata_hash: &str) -> bool {
        self.approved_metadata_hash == current_metadata_hash
    }
}

/// A stable, order-independent hash of everything a consent approval
/// actually needs to cover: the rendered text plus every provider option
/// that changes what gets posted (section 33's examples — privacy,
/// commercial disclosure, AIGC — all fold into `provider_options_json`,
/// which callers are responsible for serializing deterministically).
pub fn hash_publication_metadata(
    title: &str,
    description: &str,
    hashtags: &[String],
    provider_options_json: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(title.as_bytes());
    hasher.update([0u8]);
    hasher.update(description.as_bytes());
    hasher.update([0u8]);
    for tag in hashtags {
        hasher.update(tag.as_bytes());
        hasher.update([0u8]);
    }
    hasher.update(provider_options_json.as_bytes());
    URL_SAFE_NO_PAD.encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_inputs_hash_identically() {
        let a = hash_publication_metadata("Title", "Desc", &["#a".to_string()], "{}");
        let b = hash_publication_metadata("Title", "Desc", &["#a".to_string()], "{}");
        assert_eq!(a, b);
    }

    #[test]
    fn changing_the_caption_changes_the_hash() {
        let a = hash_publication_metadata("Title", "Desc", &[], "{}");
        let b = hash_publication_metadata("Different", "Desc", &[], "{}");
        assert_ne!(a, b);
    }

    #[test]
    fn changing_provider_options_changes_the_hash() {
        let a = hash_publication_metadata(
            "Title",
            "Desc",
            &[],
            r#"{"privacy_level":"PUBLIC_TO_EVERYONE"}"#,
        );
        let b = hash_publication_metadata("Title", "Desc", &[], r#"{"privacy_level":"SELF_ONLY"}"#);
        assert_ne!(a, b);
    }

    #[test]
    fn consent_stops_covering_metadata_once_it_changes() {
        let hash_a = hash_publication_metadata("Title", "Desc", &[], "{}");
        let consent = PublicationConsent::new(
            Uuid::new_v4(),
            Platform::TikTok,
            hash_a.clone(),
            ApprovalSource::ManualSchedule,
        );
        assert!(consent.covers(&hash_a));

        let hash_b = hash_publication_metadata("New caption", "Desc", &[], "{}");
        assert!(!consent.covers(&hash_b));
    }
}
