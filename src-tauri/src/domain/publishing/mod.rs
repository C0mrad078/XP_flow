pub mod attempt;
pub mod consent;
pub mod metadata;
pub mod publish_error;
pub mod remote_state;
pub mod retry_policy;
pub mod upload_session;

pub use attempt::{AttemptStatus, PublicationAttempt};
pub use consent::{
    hash_publication_metadata, requires_express_consent, ApprovalSource, PublicationConsent,
};
pub use metadata::{
    classify_metadata_issue, render_template, HashtagSet, MetadataTemplate,
    MetadataValidationIssue, MetadataValidationIssueCode, TemplateKind, TemplateVariables,
};
pub use publish_error::PublishError;
pub use remote_state::RemoteUploadState;
pub use upload_session::{SessionType, UploadSession};

use serde::{Deserialize, Serialize};

/// The rendered text/options actually sent to a provider for one
/// publish attempt (section 26/29) — frozen at execution time (section
/// 103) and persisted verbatim on `Publication::rendered_metadata_json`
/// so a later template edit can never retroactively change what an
/// already-executed (or in-flight) attempt claims it sent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderedMetadata {
    pub title: String,
    pub description: String,
    pub hashtags: Vec<String>,
    /// Provider-specific fields (TikTok's `privacy_level`/`disable_duet`/
    /// etc, YouTube's `category`/`privacy_status`, Kwai's cover options)
    /// as a JSON object — kept opaque here since each provider's
    /// `PlatformPublisher::validate_metadata` is the only code that needs
    /// to interpret it (section 22: "keep provider validation separate").
    pub provider_options: serde_json::Value,
}

impl RenderedMetadata {
    /// The metadata-approval hash this snapshot represents (section 33) —
    /// always derived the same way consent hashing is computed, so a
    /// consent row and a rendered snapshot never drift out of sync.
    pub fn consent_hash(&self) -> String {
        let options_json = self.provider_options.to_string();
        hash_publication_metadata(
            &self.title,
            &self.description,
            &self.hashtags,
            &options_json,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consent_hash_changes_when_provider_options_change() {
        let mut metadata = RenderedMetadata {
            title: "Title".to_string(),
            description: "Desc".to_string(),
            hashtags: vec![],
            provider_options: serde_json::json!({ "privacy_level": "PUBLIC_TO_EVERYONE" }),
        };
        let hash_a = metadata.consent_hash();
        metadata.provider_options = serde_json::json!({ "privacy_level": "SELF_ONLY" });
        let hash_b = metadata.consent_hash();
        assert_ne!(hash_a, hash_b);
    }
}
