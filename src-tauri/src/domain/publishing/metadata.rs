use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::platform::Platform;

/// A typed reason the rendered result of a metadata resolution failed a
/// provider's own validation (Phase 5.1 section 14) — the frontend
/// switches on `code`, never parses `message` text, mirroring how
/// `AppError`/`PublishError` already separate a typed code from a
/// human-readable string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MetadataValidationIssueCode {
    TitleTooLong,
    DescriptionTooLong,
    CaptionTooLong,
    MissingRequiredField,
    InvalidProviderOption,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataValidationIssue {
    pub code: MetadataValidationIssueCode,
    pub message: String,
}

/// Classifies a `PublishError::InvalidMetadata` detail string into a
/// typed code (section 14: "do not duplicate provider validation rules
/// in React" — the rules stay exactly where they already lived, inside
/// each `PlatformPublisher::validate_metadata`; this only classifies
/// *which* rule fired, via best-effort substring matching on the message
/// each check already produces, same discipline already established for
/// classifying Kwai's free-text provider errors).
pub fn classify_metadata_issue(detail: &str) -> MetadataValidationIssue {
    let lower = detail.to_lowercase();
    let code = if lower.contains("caption") {
        if lower.contains("empty") {
            MetadataValidationIssueCode::MissingRequiredField
        } else {
            MetadataValidationIssueCode::CaptionTooLong
        }
    } else if lower.contains("title") {
        if lower.contains("empty") {
            MetadataValidationIssueCode::MissingRequiredField
        } else {
            MetadataValidationIssueCode::TitleTooLong
        }
    } else if lower.contains("description") {
        MetadataValidationIssueCode::DescriptionTooLong
    } else {
        MetadataValidationIssueCode::InvalidProviderOption
    };
    MetadataValidationIssue {
        code,
        message: detail.to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateKind {
    Title,
    Description,
}

impl TemplateKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TemplateKind::Title => "title",
            TemplateKind::Description => "description",
        }
    }
}

impl std::fmt::Display for TemplateKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for TemplateKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "title" => TemplateKind::Title,
            "description" => TemplateKind::Description,
            other => return Err(format!("unknown template kind: {other}")),
        })
    }
}

/// A reusable title/description template, scoped from most to least
/// specific (section 22-25): `channel_id` + `platform` both set is the
/// most specific; both `None` is the workspace default. Precedence
/// resolution among candidates happens in
/// `application::metadata_template_service` (it needs repository lookups
/// this pure domain type doesn't have) — see section 25 for the exact
/// order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataTemplate {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub channel_id: Option<Uuid>,
    pub platform: Option<Platform>,
    pub kind: TemplateKind,
    pub template_text: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MetadataTemplate {
    pub fn new(
        workspace_id: Uuid,
        channel_id: Option<Uuid>,
        platform: Option<Platform>,
        kind: TemplateKind,
        template_text: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            channel_id,
            platform,
            kind,
            template_text: template_text.into(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// A reusable, named hashtag group (section 27) — same scoping rule as
/// [`MetadataTemplate`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashtagSet {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub channel_id: Option<Uuid>,
    pub platform: Option<Platform>,
    pub name: String,
    pub hashtags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl HashtagSet {
    pub fn new(
        workspace_id: Uuid,
        channel_id: Option<Uuid>,
        platform: Option<Platform>,
        name: impl Into<String>,
        hashtags: Vec<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            channel_id,
            platform,
            name: name.into(),
            hashtags,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Values a template's `{variable}` placeholders can resolve to (section
/// 23). Deliberately a plain struct rather than a `HashMap<String,
/// String>` — every supported variable is enumerated once here, so an
/// unsupported `{typo}` in a template is simply left untouched rather
/// than silently resolving to an empty string. New variables get added
/// here, not invented ad hoc at a call site.
#[derive(Debug, Clone, Default)]
pub struct TemplateVariables {
    /// The *publication's* own title — independently editable from the
    /// underlying video (section 7 of the Phase 5.1 brief).
    pub title: String,
    /// The underlying `Video.display_title` — distinct from `title` above
    /// so a template can reference "what this clip is called" even when
    /// the publication's own title has been customized.
    pub video_title: String,
    pub source: String,
    pub channel: String,
    pub filename: String,
    pub date: String,
    pub platform: String,
    pub hashtags: String,
}

/// Renders `template_text` by substituting every `{variable}` occurrence
/// with its value from `vars`. Pure, no I/O, no template compilation step
/// — deliberately simple (section 23/24: "no AI", and nothing here needs
/// conditionals or loops yet). An unrecognized `{placeholder}` is left in
/// the output verbatim rather than silently dropped, so a typo in a
/// template is visible in the preview instead of vanishing.
pub fn render_template(template_text: &str, vars: &TemplateVariables) -> String {
    template_text
        .replace("{title}", &vars.title)
        .replace("{video_title}", &vars.video_title)
        .replace("{source}", &vars.source)
        .replace("{channel}", &vars.channel)
        .replace("{filename}", &vars.filename)
        .replace("{date}", &vars.date)
        .replace("{platform}", &vars.platform)
        .replace("{hashtags}", &vars.hashtags)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_vars() -> TemplateVariables {
        TemplateVariables {
            title: "Amazing Goal".to_string(),
            video_title: "raw_export_042".to_string(),
            source: "cutpro-export".to_string(),
            channel: "Football BR".to_string(),
            filename: "goal_042.mp4".to_string(),
            date: "2026-09-16".to_string(),
            platform: "YouTube".to_string(),
            hashtags: "#futebol #shorts".to_string(),
        }
    }

    #[test]
    fn video_title_is_distinct_from_the_publications_own_title() {
        let rendered = render_template("{title} (source: {video_title})", &sample_vars());
        assert_eq!(rendered, "Amazing Goal (source: raw_export_042)");
    }

    #[test]
    fn substitutes_every_known_variable() {
        let rendered = render_template(
            "{title} | {channel} on {platform} ({date}) {hashtags}",
            &sample_vars(),
        );
        assert_eq!(
            rendered,
            "Amazing Goal | Football BR on YouTube (2026-09-16) #futebol #shorts"
        );
    }

    #[test]
    fn an_unknown_placeholder_is_left_untouched_not_dropped() {
        let rendered = render_template("{title} {typo}", &sample_vars());
        assert_eq!(rendered, "Amazing Goal {typo}");
    }

    #[test]
    fn a_template_with_no_placeholders_passes_through_unchanged() {
        let rendered = render_template("Static caption, no variables here.", &sample_vars());
        assert_eq!(rendered, "Static caption, no variables here.");
    }

    #[test]
    fn classifies_known_length_and_emptiness_messages() {
        assert_eq!(
            classify_metadata_issue("title exceeds YouTube's 100-character limit").code,
            MetadataValidationIssueCode::TitleTooLong
        );
        assert_eq!(
            classify_metadata_issue("title cannot be empty").code,
            MetadataValidationIssueCode::MissingRequiredField
        );
        assert_eq!(
            classify_metadata_issue("caption exceeds TikTok's 2200-character limit").code,
            MetadataValidationIssueCode::CaptionTooLong
        );
        assert_eq!(
            classify_metadata_issue("caption cannot be empty").code,
            MetadataValidationIssueCode::MissingRequiredField
        );
        assert_eq!(
            classify_metadata_issue("description exceeds YouTube's 5000-character limit").code,
            MetadataValidationIssueCode::DescriptionTooLong
        );
    }

    #[test]
    fn an_unrecognized_message_falls_back_to_invalid_provider_option() {
        assert_eq!(
            classify_metadata_issue("something provider-specific went wrong").code,
            MetadataValidationIssueCode::InvalidProviderOption
        );
    }
}
