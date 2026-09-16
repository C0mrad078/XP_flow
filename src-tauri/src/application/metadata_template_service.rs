use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform::Platform;
use crate::domain::ports::platform_publisher::PlatformPublisher;
use crate::domain::ports::repositories::{
    ChannelRepository, HashtagSetRepository, MetadataTemplateRepository, PublicationRepository,
    VideoRepository, VideoSourceRepository,
};
use crate::domain::publication::Publication;
use crate::domain::publishing::{
    classify_metadata_issue, render_template, HashtagSet, MetadataTemplate,
    MetadataValidationIssue, PublishError, RenderedMetadata, TemplateKind, TemplateVariables,
};

/// Everything the metadata editor needs to update in one publication in
/// one call (Phase 5.1 section 12/39) — a partial update: any field left
/// `None` is left exactly as it currently is.
#[derive(Debug, Default)]
pub struct MetadataUpdateInput {
    pub title: Option<String>,
    pub description: Option<String>,
    pub hashtags: Option<Vec<String>>,
    pub title_override: Option<Option<String>>,
    pub description_override: Option<Option<String>>,
    pub hashtags_override: Option<Option<Vec<String>>>,
    pub title_template_id: Option<Option<Uuid>>,
    pub description_template_id: Option<Option<Uuid>>,
    pub hashtag_set_id: Option<Option<Uuid>>,
    pub provider_options_override: Option<Option<serde_json::Value>>,
}

/// Resolves a publication's actual, provider-ready metadata deterministically
/// (Phase 5.1 section 4/5) and owns real CRUD for `MetadataTemplate`/
/// `HashtagSet`. This is the *only* place the precedence ladder is
/// implemented — `PublishingEngineService` calls `render_for_publication`
/// instead of building `RenderedMetadata` inline.
///
/// Precedence, per field (title/description independently; hashtags via
/// the analogous `HashtagSet` ladder), documented in full in
/// `docs/metadata-templates.md`:
///
/// 1. Publication override (`metadata_overrides.*_override`) — literal
///    text/list for this one publication, still rendered through
///    `{variables}`.
/// 2. A specific template/set pinned on the publication
///    (`metadata_overrides.*_template_id` / `hashtag_set_id`) — "Select
///    specific template" in the editor.
/// 3. Channel + Platform template/set.
/// 4. Channel default (channel set, no platform).
/// 5. Workspace + Platform default (no channel, platform set).
/// 6. Workspace default (no channel, no platform).
/// 7. The publication's own raw `title`/`description`/`hashtags` fields
///    — unchanged Phase 1-4 behavior, and the only rung ever reached by a
///    publication that has never touched the template system.
pub struct MetadataTemplateService {
    template_repo: Arc<dyn MetadataTemplateRepository>,
    hashtag_repo: Arc<dyn HashtagSetRepository>,
    publication_repo: Arc<dyn PublicationRepository>,
    video_repo: Arc<dyn VideoRepository>,
    video_source_repo: Arc<dyn VideoSourceRepository>,
    channel_repo: Arc<dyn ChannelRepository>,
    publishers: HashMap<Platform, Arc<dyn PlatformPublisher>>,
}

impl MetadataTemplateService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        template_repo: Arc<dyn MetadataTemplateRepository>,
        hashtag_repo: Arc<dyn HashtagSetRepository>,
        publication_repo: Arc<dyn PublicationRepository>,
        video_repo: Arc<dyn VideoRepository>,
        video_source_repo: Arc<dyn VideoSourceRepository>,
        channel_repo: Arc<dyn ChannelRepository>,
        publishers: HashMap<Platform, Arc<dyn PlatformPublisher>>,
    ) -> Self {
        Self {
            template_repo,
            hashtag_repo,
            publication_repo,
            video_repo,
            video_source_repo,
            channel_repo,
            publishers,
        }
    }

    // ---------------------------------------------------------------
    // Template CRUD
    // ---------------------------------------------------------------

    pub async fn list_templates(&self, workspace_id: Uuid) -> DomainResult<Vec<MetadataTemplate>> {
        self.template_repo.list_for_workspace(workspace_id).await
    }

    pub async fn create_template(
        &self,
        workspace_id: Uuid,
        channel_id: Option<Uuid>,
        platform: Option<Platform>,
        kind: TemplateKind,
        template_text: String,
    ) -> DomainResult<MetadataTemplate> {
        if template_text.trim().is_empty() {
            return Err(DomainError::Validation(
                "template text cannot be empty".to_string(),
            ));
        }
        self.ensure_scope_is_free(workspace_id, channel_id, platform, kind, None)
            .await?;
        let template =
            MetadataTemplate::new(workspace_id, channel_id, platform, kind, template_text);
        self.template_repo.create(&template).await?;
        Ok(template)
    }

    pub async fn update_template(
        &self,
        id: Uuid,
        channel_id: Option<Uuid>,
        platform: Option<Platform>,
        template_text: String,
    ) -> DomainResult<MetadataTemplate> {
        if template_text.trim().is_empty() {
            return Err(DomainError::Validation(
                "template text cannot be empty".to_string(),
            ));
        }
        let mut template = self
            .template_repo
            .get(id)
            .await?
            .ok_or(DomainError::NotFound {
                entity: "metadata template",
                id: id.to_string(),
            })?;
        self.ensure_scope_is_free(
            template.workspace_id,
            channel_id,
            platform,
            template.kind,
            Some(id),
        )
        .await?;
        template.channel_id = channel_id;
        template.platform = platform;
        template.template_text = template_text;
        template.updated_at = Utc::now();
        self.template_repo.update(&template).await?;
        Ok(template)
    }

    /// Safe by construction, not by extra bookkeeping here: no publication
    /// stores a foreign key into a template except through
    /// `metadata_overrides.*_template_id`, which is `ON DELETE SET NULL`
    /// at the schema level (section 13) — deleting a template can never
    /// corrupt a historical `rendered_metadata` snapshot (already fully
    /// materialized text with no reference back to any template) and
    /// never leaves a dangling pin; resolution simply falls through to
    /// the next rung for any publication that had pinned it.
    pub async fn delete_template(&self, id: Uuid) -> DomainResult<()> {
        self.template_repo.delete(id).await
    }

    async fn ensure_scope_is_free(
        &self,
        workspace_id: Uuid,
        channel_id: Option<Uuid>,
        platform: Option<Platform>,
        kind: TemplateKind,
        ignore_id: Option<Uuid>,
    ) -> DomainResult<()> {
        let existing = self.template_repo.list_for_workspace(workspace_id).await?;
        let conflict = existing.iter().any(|t| {
            Some(t.id) != ignore_id
                && t.kind == kind
                && t.channel_id == channel_id
                && t.platform == platform
        });
        if conflict {
            return Err(DomainError::Conflict(
                "a template already exists for this exact scope — edit or delete it instead of creating a duplicate".to_string(),
            ));
        }
        Ok(())
    }

    // ---------------------------------------------------------------
    // Hashtag set CRUD
    // ---------------------------------------------------------------

    pub async fn list_hashtag_sets(&self, workspace_id: Uuid) -> DomainResult<Vec<HashtagSet>> {
        self.hashtag_repo.list_for_workspace(workspace_id).await
    }

    pub async fn create_hashtag_set(
        &self,
        workspace_id: Uuid,
        channel_id: Option<Uuid>,
        platform: Option<Platform>,
        name: String,
        hashtags: Vec<String>,
    ) -> DomainResult<HashtagSet> {
        if name.trim().is_empty() {
            return Err(DomainError::Validation(
                "hashtag set name cannot be empty".to_string(),
            ));
        }
        let set = HashtagSet::new(workspace_id, channel_id, platform, name, hashtags);
        self.hashtag_repo.create(&set).await?;
        Ok(set)
    }

    pub async fn update_hashtag_set(
        &self,
        id: Uuid,
        channel_id: Option<Uuid>,
        platform: Option<Platform>,
        name: String,
        hashtags: Vec<String>,
    ) -> DomainResult<HashtagSet> {
        if name.trim().is_empty() {
            return Err(DomainError::Validation(
                "hashtag set name cannot be empty".to_string(),
            ));
        }
        let mut set = self
            .hashtag_repo
            .get(id)
            .await?
            .ok_or(DomainError::NotFound {
                entity: "hashtag set",
                id: id.to_string(),
            })?;
        set.channel_id = channel_id;
        set.platform = platform;
        set.name = name;
        set.hashtags = hashtags;
        set.updated_at = Utc::now();
        self.hashtag_repo.update(&set).await?;
        Ok(set)
    }

    /// Safe by construction, same reasoning as `delete_template`:
    /// `publications.hashtag_set_id` is `ON DELETE SET NULL`.
    pub async fn delete_hashtag_set(&self, id: Uuid) -> DomainResult<()> {
        self.hashtag_repo.delete(id).await
    }

    // ---------------------------------------------------------------
    // Resolution
    // ---------------------------------------------------------------

    /// Renders a publication's metadata without persisting or freezing
    /// anything — what the metadata editor's live preview and
    /// `preview_publication_metadata` use (section 9).
    pub async fn preview(&self, publication_id: Uuid) -> DomainResult<RenderedMetadata> {
        let publication = self.load_publication(publication_id).await?;
        self.render_for_publication(&publication).await
    }

    /// Runs the target provider's real `validate_metadata` against the
    /// current resolution and classifies the result (section 14) — the
    /// same check `PublishingEngineService::execute_inner` runs before
    /// ever transmitting anything, exposed read-only for the UI.
    pub async fn validate(
        &self,
        publication_id: Uuid,
    ) -> DomainResult<Vec<MetadataValidationIssue>> {
        let publication = self.load_publication(publication_id).await?;
        let rendered = self.render_for_publication(&publication).await?;
        let Some(publisher) = self.publishers.get(&publication.platform) else {
            return Ok(Vec::new());
        };
        match publisher.validate_metadata(&rendered) {
            Ok(()) => Ok(Vec::new()),
            Err(PublishError::InvalidMetadata { detail }) => {
                Ok(vec![classify_metadata_issue(&detail)])
            }
            Err(other) => Ok(vec![MetadataValidationIssue {
                code: crate::domain::publishing::MetadataValidationIssueCode::InvalidProviderOption,
                message: other.user_message(),
            }]),
        }
    }

    pub async fn update_publication_metadata(
        &self,
        publication_id: Uuid,
        input: MetadataUpdateInput,
    ) -> DomainResult<Publication> {
        let mut publication = self.load_publication(publication_id).await?;

        if let Some(title) = input.title {
            publication.title = title;
        }
        if let Some(description) = input.description {
            publication.description = Some(description);
        }
        if let Some(hashtags) = input.hashtags {
            publication.hashtags = hashtags;
        }
        apply_override(
            &mut publication.metadata_overrides.title_override,
            input.title_override,
        );
        apply_override(
            &mut publication.metadata_overrides.description_override,
            input.description_override,
        );
        apply_override(
            &mut publication.metadata_overrides.hashtags_override,
            input.hashtags_override,
        );
        apply_override(
            &mut publication.metadata_overrides.title_template_id,
            input.title_template_id,
        );
        apply_override(
            &mut publication.metadata_overrides.description_template_id,
            input.description_template_id,
        );
        apply_override(
            &mut publication.metadata_overrides.hashtag_set_id,
            input.hashtag_set_id,
        );
        apply_override(
            &mut publication.metadata_overrides.provider_options_override,
            input.provider_options_override,
        );

        self.publication_repo.update(&publication).await?;
        Ok(publication)
    }

    /// The one real implementation of the precedence ladder — called both
    /// by `preview`/`validate` above and by
    /// `PublishingEngineService::execute_inner` at execution time
    /// (section 10/103): the exact same resolution a live preview shows
    /// is what gets frozen onto `rendered_metadata` the moment upload
    /// starts, never a second, subtly different code path.
    pub async fn render_for_publication(
        &self,
        publication: &Publication,
    ) -> DomainResult<RenderedMetadata> {
        let video =
            self.video_repo
                .get(publication.video_id)
                .await?
                .ok_or(DomainError::NotFound {
                    entity: "video",
                    id: publication.video_id.to_string(),
                })?;
        let channel =
            self.channel_repo
                .get(publication.channel_id)
                .await?
                .ok_or(DomainError::NotFound {
                    entity: "channel",
                    id: publication.channel_id.to_string(),
                })?;
        let source_name = self
            .video_source_repo
            .get(video.source_id)
            .await?
            .map(|s| s.name)
            .unwrap_or_default();

        let mut vars = TemplateVariables {
            title: publication.title.clone(),
            video_title: video.display_title.clone(),
            source: source_name,
            channel: channel.name.clone(),
            filename: video.original_filename.clone(),
            date: publication
                .scheduled_at
                .unwrap_or_else(Utc::now)
                .format("%Y-%m-%d")
                .to_string(),
            platform: publication.platform.display_name().to_string(),
            hashtags: String::new(),
        };

        let hashtags = self.resolve_hashtags(publication, &vars).await;
        vars.hashtags = hashtags.join(" ");

        let title = self
            .resolve_field_text(
                &publication.metadata_overrides.title_override,
                publication.metadata_overrides.title_template_id,
                publication.workspace_id,
                publication.channel_id,
                publication.platform,
                TemplateKind::Title,
                &vars,
                &publication.title,
            )
            .await;
        let description = self
            .resolve_field_text(
                &publication.metadata_overrides.description_override,
                publication.metadata_overrides.description_template_id,
                publication.workspace_id,
                publication.channel_id,
                publication.platform,
                TemplateKind::Description,
                &vars,
                publication.description.as_deref().unwrap_or(""),
            )
            .await;

        let provider_options = publication
            .metadata_overrides
            .provider_options_override
            .clone()
            .unwrap_or_else(|| serde_json::json!({}));

        Ok(RenderedMetadata {
            title,
            description,
            hashtags,
            provider_options,
        })
    }

    async fn load_publication(&self, publication_id: Uuid) -> DomainResult<Publication> {
        self.publication_repo
            .get(publication_id)
            .await?
            .ok_or(DomainError::NotFound {
                entity: "publication",
                id: publication_id.to_string(),
            })
    }

    #[allow(clippy::too_many_arguments)]
    async fn resolve_field_text(
        &self,
        override_text: &Option<String>,
        pinned_template_id: Option<Uuid>,
        workspace_id: Uuid,
        channel_id: Uuid,
        platform: Platform,
        kind: TemplateKind,
        vars: &TemplateVariables,
        raw_fallback: &str,
    ) -> String {
        if let Some(text) = override_text {
            return render_template(text, vars);
        }
        if let Some(template_id) = pinned_template_id {
            if let Ok(Some(template)) = self.template_repo.get(template_id).await {
                return render_template(&template.template_text, vars);
            }
            // A pin whose target vanished despite `ON DELETE SET NULL`
            // (e.g. a stale in-memory copy) falls through rather than
            // erroring — the scope chain below is exactly the right
            // next rung to try.
        }
        if let Some(template) = self
            .resolve_template(workspace_id, channel_id, platform, kind)
            .await
        {
            return render_template(&template.template_text, vars);
        }
        render_template(raw_fallback, vars)
    }

    async fn resolve_template(
        &self,
        workspace_id: Uuid,
        channel_id: Uuid,
        platform: Platform,
        kind: TemplateKind,
    ) -> Option<MetadataTemplate> {
        let all = self
            .template_repo
            .list_for_workspace(workspace_id)
            .await
            .ok()?;
        let find = |want_channel: Option<Uuid>, want_platform: Option<Platform>| {
            all.iter()
                .find(|t| {
                    t.kind == kind && t.channel_id == want_channel && t.platform == want_platform
                })
                .cloned()
        };
        find(Some(channel_id), Some(platform))
            .or_else(|| find(Some(channel_id), None))
            .or_else(|| find(None, Some(platform)))
            .or_else(|| find(None, None))
    }

    async fn resolve_hashtags(
        &self,
        publication: &Publication,
        vars: &TemplateVariables,
    ) -> Vec<String> {
        let _ = vars; // reserved: a hashtag set's own tags are literal, not templated
        if let Some(tags) = &publication.metadata_overrides.hashtags_override {
            return tags.clone();
        }
        if let Some(set_id) = publication.metadata_overrides.hashtag_set_id {
            if let Ok(Some(set)) = self.hashtag_repo.get(set_id).await {
                return set.hashtags;
            }
        }
        if let Some(set) = self
            .resolve_hashtag_set(
                publication.workspace_id,
                publication.channel_id,
                publication.platform,
            )
            .await
        {
            return set.hashtags;
        }
        publication.hashtags.clone()
    }

    async fn resolve_hashtag_set(
        &self,
        workspace_id: Uuid,
        channel_id: Uuid,
        platform: Platform,
    ) -> Option<HashtagSet> {
        let all = self
            .hashtag_repo
            .list_for_workspace(workspace_id)
            .await
            .ok()?;
        let find = |want_channel: Option<Uuid>, want_platform: Option<Platform>| {
            all.iter()
                .find(|s| s.channel_id == want_channel && s.platform == want_platform)
                .cloned()
        };
        find(Some(channel_id), Some(platform))
            .or_else(|| find(Some(channel_id), None))
            .or_else(|| find(None, Some(platform)))
            .or_else(|| find(None, None))
    }
}

/// `Some(inner)` applies a new value (possibly clearing it back to
/// `None` via `Some(None)`); a bare `None` leaves the field untouched —
/// the standard "partial update" double-Option pattern, needed here
/// because every override field is itself an `Option`.
fn apply_override<T>(target: &mut Option<T>, input: Option<Option<T>>) {
    if let Some(value) = input {
        *target = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::video_status::VideoPriority;
    use crate::infrastructure::publishing::{FakePublisher, FakeScenario};
    use crate::infrastructure::repositories::{
        SqliteChannelRepository, SqliteHashtagSetRepository, SqliteMetadataTemplateRepository,
        SqlitePublicationRepository, SqliteVideoRepository, SqliteVideoSourceRepository,
    };
    use crate::test_support::*;

    struct Fixture {
        service: MetadataTemplateService,
        publication_repo: Arc<SqlitePublicationRepository>,
        workspace_id: Uuid,
        channel_id: Uuid,
        video_id: Uuid,
    }

    async fn build_fixture() -> Fixture {
        let pool = temp_pool("metadata-template-service").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Football Cuts").await;
        let video_id = seed_video(
            &pool,
            workspace_id,
            source_id,
            Some(channel_id),
            "Neymar talks",
        )
        .await;

        let publication_repo = Arc::new(SqlitePublicationRepository::new(pool.clone()));
        let mut publishers: HashMap<Platform, Arc<dyn PlatformPublisher>> = HashMap::new();
        publishers.insert(
            Platform::YouTube,
            Arc::new(FakePublisher::new(Platform::YouTube, FakeScenario::Success)),
        );

        let service = MetadataTemplateService::new(
            Arc::new(SqliteMetadataTemplateRepository::new(pool.clone())),
            Arc::new(SqliteHashtagSetRepository::new(pool.clone())),
            publication_repo.clone() as Arc<dyn PublicationRepository>,
            Arc::new(SqliteVideoRepository::new(pool.clone())) as Arc<dyn VideoRepository>,
            Arc::new(SqliteVideoSourceRepository::new(pool.clone()))
                as Arc<dyn VideoSourceRepository>,
            Arc::new(SqliteChannelRepository::new(pool.clone())) as Arc<dyn ChannelRepository>,
            publishers,
        );

        Fixture {
            service,
            publication_repo,
            workspace_id,
            channel_id,
            video_id,
        }
    }

    async fn seed_publication(fixture: &Fixture, title: &str) -> Uuid {
        let publication = Publication::new(
            fixture.workspace_id,
            fixture.video_id,
            fixture.channel_id,
            Platform::YouTube,
            title,
            VideoPriority::Normal,
        );
        fixture.publication_repo.create(&publication).await.unwrap();
        publication.id
    }

    #[tokio::test]
    async fn falls_back_to_the_publications_raw_title_when_nothing_else_is_configured() {
        let fixture = build_fixture().await;
        let id = seed_publication(&fixture, "Raw title").await;
        let rendered = fixture.service.preview(id).await.unwrap();
        assert_eq!(rendered.title, "Raw title");
    }

    #[tokio::test]
    async fn a_workspace_default_template_applies_when_no_more_specific_scope_matches() {
        let fixture = build_fixture().await;
        let id = seed_publication(&fixture, "Raw title").await;
        fixture
            .service
            .create_template(
                fixture.workspace_id,
                None,
                None,
                TemplateKind::Title,
                "{video_title} — {channel}".to_string(),
            )
            .await
            .unwrap();

        let rendered = fixture.service.preview(id).await.unwrap();
        assert_eq!(rendered.title, "Neymar talks — Football Cuts");
    }

    #[tokio::test]
    async fn a_workspace_platform_template_beats_a_plain_workspace_default() {
        let fixture = build_fixture().await;
        let id = seed_publication(&fixture, "Raw title").await;
        fixture
            .service
            .create_template(
                fixture.workspace_id,
                None,
                None,
                TemplateKind::Title,
                "workspace default".to_string(),
            )
            .await
            .unwrap();
        fixture
            .service
            .create_template(
                fixture.workspace_id,
                None,
                Some(Platform::YouTube),
                TemplateKind::Title,
                "workspace+youtube".to_string(),
            )
            .await
            .unwrap();

        let rendered = fixture.service.preview(id).await.unwrap();
        assert_eq!(rendered.title, "workspace+youtube");
    }

    #[tokio::test]
    async fn a_channel_default_beats_a_workspace_platform_template() {
        let fixture = build_fixture().await;
        let id = seed_publication(&fixture, "Raw title").await;
        fixture
            .service
            .create_template(
                fixture.workspace_id,
                None,
                Some(Platform::YouTube),
                TemplateKind::Title,
                "workspace+youtube".to_string(),
            )
            .await
            .unwrap();
        fixture
            .service
            .create_template(
                fixture.workspace_id,
                Some(fixture.channel_id),
                None,
                TemplateKind::Title,
                "channel default".to_string(),
            )
            .await
            .unwrap();

        let rendered = fixture.service.preview(id).await.unwrap();
        assert_eq!(rendered.title, "channel default");
    }

    #[tokio::test]
    async fn a_channel_platform_override_beats_a_channel_default() {
        let fixture = build_fixture().await;
        let id = seed_publication(&fixture, "Raw title").await;
        fixture
            .service
            .create_template(
                fixture.workspace_id,
                Some(fixture.channel_id),
                None,
                TemplateKind::Title,
                "channel default".to_string(),
            )
            .await
            .unwrap();
        fixture
            .service
            .create_template(
                fixture.workspace_id,
                Some(fixture.channel_id),
                Some(Platform::YouTube),
                TemplateKind::Title,
                "channel+youtube".to_string(),
            )
            .await
            .unwrap();

        let rendered = fixture.service.preview(id).await.unwrap();
        assert_eq!(rendered.title, "channel+youtube");
    }

    #[tokio::test]
    async fn a_publication_override_beats_every_template() {
        let fixture = build_fixture().await;
        let id = seed_publication(&fixture, "Raw title").await;
        fixture
            .service
            .create_template(
                fixture.workspace_id,
                Some(fixture.channel_id),
                Some(Platform::YouTube),
                TemplateKind::Title,
                "channel+youtube".to_string(),
            )
            .await
            .unwrap();
        fixture
            .service
            .update_publication_metadata(
                id,
                MetadataUpdateInput {
                    title_override: Some(Some("Literal override for {channel}".to_string())),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let rendered = fixture.service.preview(id).await.unwrap();
        assert_eq!(rendered.title, "Literal override for Football Cuts");
    }

    #[tokio::test]
    async fn a_pinned_specific_template_beats_the_scope_chain() {
        let fixture = build_fixture().await;
        let id = seed_publication(&fixture, "Raw title").await;
        fixture
            .service
            .create_template(
                fixture.workspace_id,
                Some(fixture.channel_id),
                Some(Platform::YouTube),
                TemplateKind::Title,
                "channel+youtube (would normally win)".to_string(),
            )
            .await
            .unwrap();
        let pinned = fixture
            .service
            .create_template(
                fixture.workspace_id,
                None,
                None,
                TemplateKind::Title,
                "explicitly pinned template".to_string(),
            )
            .await
            .unwrap();
        fixture
            .service
            .update_publication_metadata(
                id,
                MetadataUpdateInput {
                    title_template_id: Some(Some(pinned.id)),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let rendered = fixture.service.preview(id).await.unwrap();
        assert_eq!(rendered.title, "explicitly pinned template");
    }

    #[tokio::test]
    async fn deleting_a_pinned_template_falls_back_to_the_next_rung_not_an_error() {
        let fixture = build_fixture().await;
        let id = seed_publication(&fixture, "Raw title").await;
        let pinned = fixture
            .service
            .create_template(
                fixture.workspace_id,
                None,
                None,
                TemplateKind::Title,
                "pinned".to_string(),
            )
            .await
            .unwrap();
        fixture
            .service
            .update_publication_metadata(
                id,
                MetadataUpdateInput {
                    title_template_id: Some(Some(pinned.id)),
                    ..Default::default()
                },
            )
            .await
            .unwrap();
        fixture.service.delete_template(pinned.id).await.unwrap();

        let rendered = fixture.service.preview(id).await.unwrap();
        assert_eq!(rendered.title, "Raw title");
    }

    #[tokio::test]
    async fn an_unrecognized_placeholder_is_left_untouched_not_silently_dropped() {
        let fixture = build_fixture().await;
        let id = seed_publication(&fixture, "Raw title").await;
        fixture
            .service
            .create_template(
                fixture.workspace_id,
                None,
                None,
                TemplateKind::Title,
                "{video_title} {not_a_real_variable}".to_string(),
            )
            .await
            .unwrap();

        let rendered = fixture.service.preview(id).await.unwrap();
        assert_eq!(rendered.title, "Neymar talks {not_a_real_variable}");
    }

    #[tokio::test]
    async fn a_hashtag_set_resolves_and_can_be_referenced_from_a_title_template() {
        let fixture = build_fixture().await;
        let id = seed_publication(&fixture, "Raw title").await;
        fixture
            .service
            .create_hashtag_set(
                fixture.workspace_id,
                Some(fixture.channel_id),
                None,
                "Football BR".to_string(),
                vec!["#futebol".to_string(), "#shorts".to_string()],
            )
            .await
            .unwrap();
        fixture
            .service
            .create_template(
                fixture.workspace_id,
                None,
                None,
                TemplateKind::Title,
                "{video_title} {hashtags}".to_string(),
            )
            .await
            .unwrap();

        let rendered = fixture.service.preview(id).await.unwrap();
        assert_eq!(rendered.hashtags, vec!["#futebol", "#shorts"]);
        assert_eq!(rendered.title, "Neymar talks #futebol #shorts");
    }

    #[tokio::test]
    async fn creating_a_second_template_for_the_same_exact_scope_is_rejected() {
        let fixture = build_fixture().await;
        fixture
            .service
            .create_template(
                fixture.workspace_id,
                None,
                None,
                TemplateKind::Title,
                "first".to_string(),
            )
            .await
            .unwrap();

        let result = fixture
            .service
            .create_template(
                fixture.workspace_id,
                None,
                None,
                TemplateKind::Title,
                "second".to_string(),
            )
            .await;
        assert!(matches!(result, Err(DomainError::Conflict(_))));
    }

    #[tokio::test]
    async fn validate_classifies_a_title_length_violation() {
        // FakePublisher only checks emptiness (see its own doc comment) —
        // exercising a real length limit needs a real uploader's
        // validate_metadata, so this rebuilds the fixture with
        // YouTubeUploader (100-char title limit) instead.
        let pool = temp_pool("metadata-template-service-validate").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Football Cuts").await;
        let video_id = seed_video(
            &pool,
            workspace_id,
            source_id,
            Some(channel_id),
            "Neymar talks",
        )
        .await;
        let publication_repo = Arc::new(SqlitePublicationRepository::new(pool.clone()));
        let mut publishers: HashMap<Platform, Arc<dyn PlatformPublisher>> = HashMap::new();
        publishers.insert(
            Platform::YouTube,
            Arc::new(crate::infrastructure::connectors::youtube::YouTubeUploader::new()),
        );
        let service = MetadataTemplateService::new(
            Arc::new(SqliteMetadataTemplateRepository::new(pool.clone())),
            Arc::new(SqliteHashtagSetRepository::new(pool.clone())),
            publication_repo.clone() as Arc<dyn PublicationRepository>,
            Arc::new(SqliteVideoRepository::new(pool.clone())) as Arc<dyn VideoRepository>,
            Arc::new(SqliteVideoSourceRepository::new(pool.clone()))
                as Arc<dyn VideoSourceRepository>,
            Arc::new(SqliteChannelRepository::new(pool.clone())) as Arc<dyn ChannelRepository>,
            publishers,
        );
        let publication = Publication::new(
            workspace_id,
            video_id,
            channel_id,
            Platform::YouTube,
            "x".repeat(200),
            VideoPriority::Normal,
        );
        publication_repo.create(&publication).await.unwrap();

        let issues = service.validate(publication.id).await.unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(
            issues[0].code,
            crate::domain::publishing::MetadataValidationIssueCode::TitleTooLong
        );
    }
}
