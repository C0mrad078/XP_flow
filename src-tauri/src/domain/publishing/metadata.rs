use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::platform::Platform;

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
    pub title: String,
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
            source: "cutpro-export".to_string(),
            channel: "Football BR".to_string(),
            filename: "goal_042.mp4".to_string(),
            date: "2026-09-16".to_string(),
            platform: "YouTube".to_string(),
            hashtags: "#futebol #shorts".to_string(),
        }
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
}
