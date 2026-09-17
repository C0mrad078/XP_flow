use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use chrono::{Duration, Utc};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::application::activity_service::ActivityService;
use crate::application::credential_acquisition_service::CredentialAcquisitionService;
use crate::application::provider_rate_limit_service::ProviderRateLimitService;
use crate::application::settings_service::SettingsService;
use crate::domain::activity_event::{ActivityCategory, ActivityLevel};
use crate::domain::app_settings::MissedSchedulePolicy;
use crate::domain::channel::ChannelStatus;
use crate::domain::platform::Platform;
use crate::domain::ports::hashing::ContentHashService;
use crate::domain::ports::platform_publisher::{CancelSignal, PlatformPublisher};
use crate::domain::ports::progress_publisher::ProgressPublisher;
use crate::domain::ports::repositories::{
    ChannelRepository, ExecutionStateUpdate, PlatformAccountRepository,
    PublicationAttemptRepository, PublicationConsentRepository, PublicationRepository,
    UploadSessionRepository, VideoRepository,
};
use crate::domain::publication::{Publication, PublicationStatus};
use crate::domain::publishing::retry_policy::{attempts_exhausted, next_retry_at};
use crate::domain::publishing::{
    requires_express_consent, PublicationAttempt, PublishError, RateLimitOperation,
    RemoteUploadState, RenderedMetadata,
};
use crate::services::notification_service::NotificationService;

/// How long a claim is valid before it's considered abandoned (section
/// 13). Generous relative to how long a real upload realistically takes
/// — a crash mid-upload is recovered correctly regardless (via
/// `recover_upload`/`PlatformPublisher::recover_upload`, which inspects
/// actual remote state rather than blindly restarting), so this bound
/// exists to reclaim genuinely abandoned claims, not to race a slow but
/// healthy upload.
const CLAIM_LEASE_DURATION: Duration = Duration::minutes(45);

/// Walks `Publication`'s real state machine (`domain::publication::
/// PublicationStatus::allowed_next`) from wherever a publication
/// currently sits to an immediately-due `Scheduled` state, for
/// `publish_now`. Deliberately reuses every existing transition edge
/// (the same ones `requeue_for_retry` and the scheduler already rely on)
/// rather than special-casing "force" as a bypass of the state machine —
/// a status this can't reach validly (mid-execution, terminal, or not yet
/// past content validation) is refused, not overridden.
fn force_schedule_now(
    publication: &mut Publication,
    now: chrono::DateTime<Utc>,
) -> Result<(), PublishError> {
    let invalid = |p: &Publication| PublishError::Internal {
        detail: format!("cannot publish now from status {}", p.status),
    };
    match publication.status {
        PublicationStatus::Scheduled => {}
        PublicationStatus::Queued | PublicationStatus::Paused => {
            publication
                .transition(PublicationStatus::Scheduled)
                .map_err(|_| invalid(publication))?;
        }
        PublicationStatus::AuthRequired => {
            publication
                .transition(PublicationStatus::Queued)
                .map_err(|_| invalid(publication))?;
            publication
                .transition(PublicationStatus::Scheduled)
                .map_err(|_| invalid(publication))?;
        }
        PublicationStatus::Failed | PublicationStatus::RateLimited => {
            publication
                .transition(PublicationStatus::RetryWait)
                .map_err(|_| invalid(publication))?;
            publication
                .transition(PublicationStatus::Queued)
                .map_err(|_| invalid(publication))?;
            publication
                .transition(PublicationStatus::Scheduled)
                .map_err(|_| invalid(publication))?;
        }
        _ => return Err(invalid(publication)),
    }
    publication.scheduled_at = Some(now);
    Ok(())
}

/// Orchestrates one publication's execution end to end (section 3-14):
/// claim -> credential acquisition -> provider dispatch -> attempt/
/// session persistence -> status transition. Every write to a
/// publication's execution state goes through the repository's guarded
/// methods (`try_claim_due`, `update_execution_state`,
/// `try_finish_processing`) — this service never calls the generic
/// `update()` on a publication it's actively executing.
pub struct PublishingEngineService {
    publication_repo: Arc<dyn PublicationRepository>,
    attempt_repo: Arc<dyn PublicationAttemptRepository>,
    session_repo: Arc<dyn UploadSessionRepository>,
    video_repo: Arc<dyn VideoRepository>,
    platform_account_repo: Arc<dyn PlatformAccountRepository>,
    channel_repo: Arc<dyn ChannelRepository>,
    consent_repo: Arc<dyn PublicationConsentRepository>,
    content_hash_service: Arc<dyn ContentHashService>,
    credential_service: Arc<CredentialAcquisitionService>,
    metadata_service: Arc<crate::application::metadata_template_service::MetadataTemplateService>,
    rate_limit_service: Arc<ProviderRateLimitService>,
    settings_service: Arc<SettingsService>,
    progress_publisher: Arc<dyn ProgressPublisher>,
    publishers: HashMap<Platform, Arc<dyn PlatformPublisher>>,
    activity_service: Arc<ActivityService>,
    notification_service: Arc<NotificationService>,
}

#[allow(clippy::too_many_arguments)]
impl PublishingEngineService {
    pub fn new(
        publication_repo: Arc<dyn PublicationRepository>,
        attempt_repo: Arc<dyn PublicationAttemptRepository>,
        session_repo: Arc<dyn UploadSessionRepository>,
        video_repo: Arc<dyn VideoRepository>,
        platform_account_repo: Arc<dyn PlatformAccountRepository>,
        channel_repo: Arc<dyn ChannelRepository>,
        consent_repo: Arc<dyn PublicationConsentRepository>,
        content_hash_service: Arc<dyn ContentHashService>,
        credential_service: Arc<CredentialAcquisitionService>,
        metadata_service: Arc<
            crate::application::metadata_template_service::MetadataTemplateService,
        >,
        rate_limit_service: Arc<ProviderRateLimitService>,
        settings_service: Arc<SettingsService>,
        progress_publisher: Arc<dyn ProgressPublisher>,
        publishers: HashMap<Platform, Arc<dyn PlatformPublisher>>,
        activity_service: Arc<ActivityService>,
        notification_service: Arc<NotificationService>,
    ) -> Self {
        Self {
            publication_repo,
            attempt_repo,
            session_repo,
            video_repo,
            platform_account_repo,
            channel_repo,
            consent_repo,
            content_hash_service,
            credential_service,
            metadata_service,
            rate_limit_service,
            settings_service,
            progress_publisher,
            publishers,
            activity_service,
            notification_service,
        }
    }

    /// Scans due publications and atomically claims every one whose
    /// channel isn't paused (section 82) and isn't locked (already
    /// enforced by `try_claim_due`'s own SQL). Returns the claim tokens
    /// the caller (`JobRunner`) needs to actually execute each one —
    /// claiming and executing are deliberately separate steps so a
    /// caller can enqueue a real `Job` row (idempotency/crash bookkeeping)
    /// in between (section 14/122).
    pub async fn scan_and_claim_due(&self, workspace_id: Uuid) -> Vec<(Uuid, String)> {
        let now = Utc::now();
        // Section 55/58: a global disable/pause stops the periodic scan
        // from claiming anything new — an upload already in flight keeps
        // running to whatever terminal state it reaches on its own
        // (never a fake instant cancellation), but nothing new starts.
        let publishing_settings = self
            .settings_service
            .get()
            .await
            .map(|s| s.publishing)
            .unwrap_or_default();
        if !publishing_settings.enabled || publishing_settings.paused {
            return Vec::new();
        }

        let due = match self.publication_repo.list_due(workspace_id, now).await {
            Ok(due) => due,
            Err(_) => return Vec::new(),
        };

        let mut claimed = Vec::new();
        for publication in due {
            match self.channel_repo.get(publication.channel_id).await {
                Ok(Some(channel)) if channel.status == ChannelStatus::Paused => continue,
                Ok(Some(_)) => {}
                _ => continue,
            }

            // Section 55/57: a publication overdue beyond the configured
            // grace period follows the workspace's missed-schedule
            // policy instead of being claimed as if it were merely a
            // little late.
            let overdue_minutes = publication
                .scheduled_at
                .map(|at| (now - at).num_minutes())
                .unwrap_or(0);
            if overdue_minutes > publishing_settings.missed_schedule_grace_period_minutes as i64 {
                match publishing_settings.missed_schedule_policy {
                    MissedSchedulePolicy::PublishWithinGrace
                    | MissedSchedulePolicy::NeedsReview => {
                        // Never claimed while beyond grace under either
                        // policy — PublishWithinGrace only ever
                        // auto-publishes *within* the window; beyond it,
                        // both policies leave the row `Scheduled` for a
                        // human to see (via readiness/UI), never
                        // force-publishing very stale content.
                        continue;
                    }
                    MissedSchedulePolicy::Skip => {
                        self.skip_missed_publication(&publication).await;
                        continue;
                    }
                }
            }

            let claim_token = Uuid::new_v4().to_string();
            let candidate_execution_key = publication.execution_key.unwrap_or_else(Uuid::new_v4);
            let lease_expires_at = now + CLAIM_LEASE_DURATION;
            if self
                .publication_repo
                .try_claim_due(
                    publication.id,
                    candidate_execution_key,
                    &claim_token,
                    lease_expires_at,
                    now,
                )
                .await
                .unwrap_or(false)
            {
                claimed.push((publication.id, claim_token));
            }
        }
        claimed
    }

    /// Executes one already-claimed publication. Never panics on a
    /// provider/network failure — every error path releases the claim
    /// and records what happened; the only thing this can't recover from
    /// gracefully is the process being killed mid-call, which is exactly
    /// what the startup/periodic recovery pass (`recover_interrupted`)
    /// exists for.
    pub async fn execute(&self, publication_id: Uuid, claim_token: String) {
        if let Err(err) = self.execute_inner(publication_id, &claim_token).await {
            tracing::warn!(publication_id = %publication_id, error = %err, "publication execution ended in error");
        }
    }

    /// Section 85: a user-initiated "Publish Now" — forces an otherwise
    /// valid publication (not locked, not mid-execution, its channel not
    /// paused) into an immediately-due `Scheduled` state and executes it
    /// through the exact same claim-then-execute path as the periodic
    /// scan, so it gets the same exactly-once guarantee — this is never a
    /// separate, parallel execution path.
    pub async fn publish_now(&self, publication_id: Uuid) -> Result<(), PublishError> {
        // Section 58: a global pause is a hard stop — "Publish Now" is
        // not a backdoor around it. Resume first, then publish.
        let publishing_settings = self
            .settings_service
            .get()
            .await
            .map(|s| s.publishing)
            .unwrap_or_default();
        if !publishing_settings.enabled || publishing_settings.paused {
            return Err(PublishError::Internal {
                detail: "publishing is currently paused for this workspace".to_string(),
            });
        }

        let mut publication = self
            .publication_repo
            .get(publication_id)
            .await
            .map_err(|e| PublishError::Internal {
                detail: e.to_string(),
            })?
            .ok_or_else(|| PublishError::Internal {
                detail: "publication not found".to_string(),
            })?;

        if publication.locked {
            return Err(PublishError::Internal {
                detail: "publication is locked".to_string(),
            });
        }
        match self.channel_repo.get(publication.channel_id).await {
            Ok(Some(channel)) if channel.status == ChannelStatus::Paused => {
                return Err(PublishError::Internal {
                    detail: "the channel is paused".to_string(),
                });
            }
            Ok(Some(_)) => {}
            _ => {
                return Err(PublishError::Internal {
                    detail: "channel not found".to_string(),
                })
            }
        }

        let now = Utc::now();
        force_schedule_now(&mut publication, now)?;
        self.publication_repo
            .update(&publication)
            .await
            .map_err(|e| PublishError::Internal {
                detail: e.to_string(),
            })?;

        let claim_token = Uuid::new_v4().to_string();
        let candidate_execution_key = publication.execution_key.unwrap_or_else(Uuid::new_v4);
        let lease_expires_at = now + CLAIM_LEASE_DURATION;
        let claimed = self
            .publication_repo
            .try_claim_due(
                publication.id,
                candidate_execution_key,
                &claim_token,
                lease_expires_at,
                now,
            )
            .await
            .map_err(|e| PublishError::Internal {
                detail: e.to_string(),
            })?;
        if !claimed {
            return Err(PublishError::Internal {
                detail: "publication could not be claimed for immediate execution".to_string(),
            });
        }

        self.execute_inner(publication.id, &claim_token).await
    }

    /// Durable, never-overwritten attempt history for the Publication
    /// Details drawer (section 90/94) — newest first.
    pub async fn get_attempts(&self, publication_id: Uuid) -> Vec<PublicationAttempt> {
        self.attempt_repo
            .list_for_publication(publication_id)
            .await
            .unwrap_or_default()
    }

    /// Records explicit user approval of the publication's *current*
    /// rendered metadata (section 31-34) — the only way a TikTok
    /// publication's express-consent gate is ever satisfied. Always
    /// inserts a fresh row rather than mutating one in place: the
    /// approval trail stays a complete, auditable history, and
    /// `PublicationConsentRepository::latest_for_publication` is what the
    /// engine actually checks.
    pub async fn record_consent(
        &self,
        publication_id: Uuid,
        approval_source: crate::domain::publishing::ApprovalSource,
    ) -> Result<(), PublishError> {
        let publication = self
            .publication_repo
            .get(publication_id)
            .await
            .map_err(|e| PublishError::Internal {
                detail: e.to_string(),
            })?
            .ok_or_else(|| PublishError::Internal {
                detail: "publication not found".to_string(),
            })?;

        let metadata = self
            .metadata_service
            .render_for_publication(&publication)
            .await
            .map_err(|e| PublishError::Internal {
                detail: e.to_string(),
            })?;
        let consent = crate::domain::publishing::PublicationConsent::new(
            publication_id,
            publication.platform,
            metadata.consent_hash(),
            approval_source,
        );
        self.consent_repo
            .create(&consent)
            .await
            .map_err(|e| PublishError::Internal {
                detail: e.to_string(),
            })
    }

    async fn execute_inner(
        &self,
        publication_id: Uuid,
        claim_token: &str,
    ) -> Result<(), PublishError> {
        let publication = self
            .publication_repo
            .get(publication_id)
            .await
            .map_err(|e| PublishError::Internal {
                detail: e.to_string(),
            })?
            .ok_or_else(|| PublishError::Internal {
                detail: "publication not found".to_string(),
            })?;

        let video = self
            .video_repo
            .get(publication.video_id)
            .await
            .map_err(|e| PublishError::Internal {
                detail: e.to_string(),
            })?
            .ok_or(PublishError::VideoUnavailable)?;

        // Section 117/118: the source file is re-verified immediately
        // before use — never trusted just because it was valid when
        // ingested or queued.
        let path = Path::new(&video.file_path);
        if !path.exists() {
            return self
                .fail_and_release(&publication, claim_token, PublishError::VideoUnavailable, 0)
                .await;
        }
        if let Some(expected_hash) = &video.content_hash {
            match self.content_hash_service.hash_file(path).await {
                Ok(actual_hash) if &actual_hash != expected_hash => {
                    return self
                        .fail_and_release(
                            &publication,
                            claim_token,
                            PublishError::InvalidMedia {
                                detail: "the source file changed since it was queued".to_string(),
                            },
                            0,
                        )
                        .await;
                }
                _ => {}
            }
        }

        let account = match publication.platform_account_id {
            Some(id) => self.platform_account_repo.get(id).await.ok().flatten(),
            None => None,
        };
        let Some(account) = account else {
            return self
                .fail_and_release(&publication, claim_token, PublishError::AuthExpired, 0)
                .await;
        };
        // Section 21: "account connected" and "account can publish" are
        // different claims — a missing scope is caught here, before any
        // network call, rather than discovered as an opaque provider
        // rejection mid-upload.
        if !account.has_capability(crate::domain::capability::Capability::UploadVideo) {
            return self
                .fail_and_release(
                    &publication,
                    claim_token,
                    PublishError::PermissionMissing {
                        capability: "upload_video".to_string(),
                    },
                    0,
                )
                .await;
        }

        // Section 22: never repeatedly hit a known rate limit — checked
        // before any provider call, including before rendering metadata.
        if !self
            .rate_limit_service
            .can_execute(account.id, RateLimitOperation::Publish)
            .await
        {
            let retry_after_seconds = self
                .rate_limit_service
                .get_next_allowed_at(account.id, RateLimitOperation::Publish)
                .await
                .map(|at| (at - Utc::now()).num_seconds().max(0) as u64);
            return self
                .fail_and_release(
                    &publication,
                    claim_token,
                    PublishError::RateLimited {
                        retry_after_seconds,
                    },
                    0,
                )
                .await;
        }

        let publisher = self
            .publishers
            .get(&publication.platform)
            .ok_or_else(|| PublishError::Internal {
                detail: format!("no publisher registered for {}", publication.platform),
            })?
            .clone();

        // Section 22/29/103: resolved through the real precedence ladder
        // (`MetadataTemplateService::render_for_publication`) — the exact
        // same resolution a live preview shows is what gets frozen the
        // moment execution starts, never a second, subtly different
        // inline computation. Routed through `fail_and_release` on error,
        // same as every other pre-flight check here — a resolution
        // failure (e.g. the underlying video/channel row vanished) must
        // still release the claim, never leave it stuck `Uploading`.
        let metadata = match self
            .metadata_service
            .render_for_publication(&publication)
            .await
        {
            Ok(metadata) => metadata,
            Err(e) => {
                return self
                    .fail_and_release(
                        &publication,
                        claim_token,
                        PublishError::Internal {
                            detail: e.to_string(),
                        },
                        0,
                    )
                    .await
            }
        };

        if let Err(err) = publisher.validate_metadata(&metadata) {
            return self
                .fail_and_release(&publication, claim_token, err, 0)
                .await;
        }
        if let Err(err) = publisher.validate_media(&video).await {
            return self
                .fail_and_release(&publication, claim_token, err, 0)
                .await;
        }

        // Section 31-34: providers that require express approval (TikTok)
        // are checked against the *current* rendered metadata's hash —
        // an approval recorded against different text/options doesn't
        // count (section 33), and there is no separate "invalidated"
        // flag to forget to flip.
        if requires_express_consent(publication.platform) {
            let current_hash = metadata.consent_hash();
            let covers = self
                .consent_repo
                .latest_for_publication(publication_id)
                .await
                .ok()
                .flatten()
                .is_some_and(|consent| consent.covers(&current_hash));
            if !covers {
                return self
                    .fail_and_release(&publication, claim_token, PublishError::ConsentRequired, 0)
                    .await;
            }
        }

        self.freeze_metadata(publication_id, claim_token, &metadata)
            .await;

        let attempt_number = self
            .attempt_repo
            .max_attempt_number(publication_id)
            .await
            .unwrap_or(0)
            + 1;
        let mut attempt =
            PublicationAttempt::new(publication_id, attempt_number, publication.platform);
        attempt.start();
        let _ = self.attempt_repo.create(&attempt).await;

        let access_token = match self.credential_service.acquire(&account).await {
            Ok(token) => token,
            Err(err) => {
                if let PublishError::RateLimited {
                    retry_after_seconds,
                } = &err
                {
                    let _ = self
                        .rate_limit_service
                        .record_rate_limited(
                            account.id,
                            RateLimitOperation::Auth,
                            *retry_after_seconds,
                        )
                        .await;
                }
                return self
                    .finish_attempt_failure(&publication, claim_token, &mut attempt, err)
                    .await;
            }
        };

        let mut session = match publisher
            .initialize_upload(&account, &access_token, &video, &metadata)
            .await
        {
            Ok(mut session) => {
                session.publication_id = publication_id;
                session.attempt_id = attempt.id;
                session
            }
            Err(err) => {
                return self
                    .finish_attempt_failure(&publication, claim_token, &mut attempt, err)
                    .await
            }
        };
        let _ = self.session_repo.create(&session).await;

        let (progress_tx, mut progress_rx): (
            crate::domain::ports::platform_publisher::ProgressSender,
            _,
        ) = mpsc::unbounded_channel();
        // Section 90/91/28: forwarded live to the frontend event bus as
        // each chunk is acknowledged; nothing here persists a row per
        // event (section 149) — the durable checkpoint is the session
        // update below, once, after the transfer finishes rather than
        // per byte. Progress is inherently transient (section 29): the
        // frontend never treats one of these as a final result.
        let progress_publisher = self.progress_publisher.clone();
        let progress_publication_id = publication_id;
        let progress_attempt_id = attempt.id;
        let progress_platform = publication.platform;
        tokio::spawn(async move {
            while let Some(update) = progress_rx.recv().await {
                let percentage = update.bytes_total.and_then(|total| {
                    if total > 0 {
                        Some((update.bytes_uploaded as f64 / total as f64) * 100.0)
                    } else {
                        None
                    }
                });
                progress_publisher.publish(
                    crate::domain::ports::progress_publisher::PublishProgressEvent {
                        publication_id: progress_publication_id,
                        attempt_id: progress_attempt_id,
                        platform: progress_platform,
                        bytes_uploaded: update.bytes_uploaded,
                        bytes_total: update.bytes_total,
                        percentage,
                        phase: crate::domain::ports::progress_publisher::PublishProgressPhase::Uploading,
                    },
                );
            }
        });

        let (updated_session, upload_result) = publisher
            .upload_media(
                &access_token,
                session,
                &video,
                progress_tx,
                CancelSignal::new(),
            )
            .await;
        session = updated_session;
        let _ = self.session_repo.update(&session).await;
        if let Err(err) = upload_result {
            return self
                .finish_attempt_failure(&publication, claim_token, &mut attempt, err)
                .await;
        }

        let pre_finalize_session = session.clone();
        session = match publisher.finalize_publication(&access_token, session).await {
            Ok(session) => session,
            Err(err) => {
                let _ = self.session_repo.update(&pre_finalize_session).await;
                return self
                    .finish_attempt_failure(&publication, claim_token, &mut attempt, err)
                    .await;
            }
        };
        let _ = self.session_repo.update(&session).await;

        // Reaching here means initialize/upload/finalize all completed
        // without the provider ever reporting a rate limit — clear any
        // stale window so a resolved limit doesn't keep blocking future
        // attempts (section 19/85).
        if let Some(account_id) = publication.platform_account_id {
            let _ = self
                .rate_limit_service
                .record_success(account_id, RateLimitOperation::Publish)
                .await;
        }

        match session.state {
            RemoteUploadState::RemoteSucceeded => {
                attempt.succeed(session.remote_publish_id.clone());
                let _ = self.attempt_repo.update(&attempt).await;
                let released = self
                    .publication_repo
                    .update_execution_state(
                        publication_id,
                        claim_token,
                        &ExecutionStateUpdate {
                            status: PublicationStatus::Published,
                            remote_id: session.remote_publish_id.clone(),
                            retry_count: publication.retry_count,
                            last_error: None,
                            last_error_code: None,
                            rendered_metadata_json: None,
                            release_claim: true,
                            new_lease_expires_at: None,
                            published_at: Some(Utc::now()),
                        },
                    )
                    .await
                    .unwrap_or(false);
                if released {
                    self.log_success(&publication).await;
                }
                Ok(())
            }
            RemoteUploadState::RemoteProcessing | RemoteUploadState::Transferred => {
                // Section 45/57: transferred is not the same claim as
                // published — a separate poll (`poll_processing`) confirms
                // the provider's own remote result before this ever
                // becomes `Published`.
                let _ = self
                    .publication_repo
                    .update_execution_state(
                        publication_id,
                        claim_token,
                        &ExecutionStateUpdate {
                            status: PublicationStatus::Processing,
                            remote_id: session.remote_publish_id.clone(),
                            retry_count: publication.retry_count,
                            last_error: None,
                            last_error_code: None,
                            rendered_metadata_json: None,
                            release_claim: true,
                            new_lease_expires_at: None,
                            published_at: None,
                        },
                    )
                    .await;
                let _ = self
                    .activity_service
                    .log(
                        ActivityCategory::Publication,
                        ActivityLevel::Info,
                        format!(
                            "\"{}\" uploaded — waiting on {} processing",
                            publication.title, publication.platform
                        ),
                    )
                    .await;
                Ok(())
            }
            _ => {
                self.finish_attempt_failure(
                    &publication,
                    claim_token,
                    &mut attempt,
                    PublishError::UnknownRemoteResult,
                )
                .await
            }
        }
    }

    /// Every currently-`Processing` publication in the workspace — the
    /// candidate list `spawn_periodic_processing_poll` iterates. A plain
    /// filtered list, not a claim: polling doesn't need exclusivity (see
    /// `try_finish_processing`'s doc comment).
    pub async fn list_processing(&self, workspace_id: Uuid) -> Vec<Uuid> {
        let mut query = crate::domain::publication_query::PublicationListQuery::new(workspace_id);
        query.statuses = Some(vec![PublicationStatus::Processing]);
        query.page_size = 500;
        match self.publication_repo.list_paginated(&query).await {
            Ok(page) => page.items.into_iter().map(|p| p.id).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Polls a `Processing` publication's remote status (section 45/56).
    /// Read-only until a terminal result appears — never re-uploads.
    pub async fn poll_processing(&self, publication_id: Uuid) -> Result<(), PublishError> {
        let publication = self
            .publication_repo
            .get(publication_id)
            .await
            .map_err(|e| PublishError::Internal {
                detail: e.to_string(),
            })?
            .ok_or_else(|| PublishError::Internal {
                detail: "publication not found".to_string(),
            })?;
        if publication.status != PublicationStatus::Processing {
            return Ok(());
        }
        let Some(session) = self
            .session_repo
            .latest_for_publication(publication_id)
            .await
            .map_err(|e| PublishError::Internal {
                detail: e.to_string(),
            })?
        else {
            return Ok(());
        };
        let Some(account) = (match publication.platform_account_id {
            Some(id) => self.platform_account_repo.get(id).await.ok().flatten(),
            None => None,
        }) else {
            return Ok(());
        };
        let Some(publisher) = self.publishers.get(&publication.platform) else {
            return Ok(());
        };
        if !self
            .rate_limit_service
            .can_execute(account.id, RateLimitOperation::Status)
            .await
        {
            // Never poll a known-limited status endpoint again before its
            // window clears (section 22) — simply skip this tick; the
            // next periodic poll tries again automatically.
            return Ok(());
        }
        let access_token = self.credential_service.acquire(&account).await?;
        let state = match publisher.get_remote_status(&access_token, &session).await {
            Ok(state) => state,
            Err(err) => {
                if let PublishError::RateLimited {
                    retry_after_seconds,
                } = &err
                {
                    let _ = self
                        .rate_limit_service
                        .record_rate_limited(
                            account.id,
                            RateLimitOperation::Status,
                            *retry_after_seconds,
                        )
                        .await;
                }
                return Err(err);
            }
        };
        let _ = self
            .rate_limit_service
            .record_success(account.id, RateLimitOperation::Status)
            .await;

        match state {
            RemoteUploadState::RemoteSucceeded => {
                self.publication_repo
                    .try_finish_processing(
                        publication_id,
                        PublicationStatus::Published,
                        session.remote_publish_id.clone(),
                        None,
                        None,
                        Some(Utc::now()),
                    )
                    .await
                    .map_err(|e| PublishError::Internal {
                        detail: e.to_string(),
                    })?;
                self.log_success(&publication).await;
            }
            RemoteUploadState::RemoteFailed => {
                let remote_processing_failed = PublishError::RemoteProcessingFailed {
                    detail: "the platform failed to process the upload".to_string(),
                };
                self.publication_repo
                    .try_finish_processing(
                        publication_id,
                        PublicationStatus::Failed,
                        None,
                        Some(remote_processing_failed.user_message()),
                        Some(remote_processing_failed.code().to_string()),
                        None,
                    )
                    .await
                    .map_err(|e| PublishError::Internal {
                        detail: e.to_string(),
                    })?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Reconciles publications whose claim lease expired — an abandoned
    /// claim from a process that was killed mid-upload (section 13/88).
    /// Always inspects real persisted state before touching anything;
    /// never blindly restarts (section 42/43/55).
    pub async fn recover_interrupted(&self, workspace_id: Uuid) {
        let expired = match self
            .publication_repo
            .list_with_expired_leases(workspace_id, Utc::now())
            .await
        {
            Ok(rows) => rows,
            Err(_) => return,
        };
        for publication in expired {
            self.recover_one(publication).await;
        }
    }

    async fn recover_one(&self, publication: Publication) {
        let Some(claim_token) = publication.claim_token.clone() else {
            return;
        };
        let session = self
            .session_repo
            .latest_for_publication(publication.id)
            .await
            .ok()
            .flatten();

        let safe_to_restart = session
            .as_ref()
            .map(|s| s.state.safe_to_restart())
            .unwrap_or(true);
        if safe_to_restart {
            // Nothing was ever durably transferred — release the claim
            // and send it back through the retry chain so the next scan
            // can pick it up again. Releasing here (rather than via
            // `requeue_for_retry`, which expects an already-`Failed` row)
            // needs the claim to actually be dropped first.
            let _ = self
                .publication_repo
                .update_execution_state(
                    publication.id,
                    &claim_token,
                    &ExecutionStateUpdate {
                        status: PublicationStatus::Failed,
                        remote_id: None,
                        retry_count: publication.retry_count,
                        last_error: Some("interrupted before any data was sent".to_string()),
                        last_error_code: None,
                        rendered_metadata_json: None,
                        release_claim: true,
                        new_lease_expires_at: None,
                        published_at: None,
                    },
                )
                .await;
            self.requeue_for_retry(&publication, publication.retry_count, None)
                .await;
            return;
        }

        let Some(session) = session else { return };
        let Some(video) = self
            .video_repo
            .get(publication.video_id)
            .await
            .ok()
            .flatten()
        else {
            return;
        };
        let Some(account) = (match publication.platform_account_id {
            Some(id) => self.platform_account_repo.get(id).await.ok().flatten(),
            None => None,
        }) else {
            return;
        };
        let Some(publisher) = self.publishers.get(&publication.platform) else {
            return;
        };
        let Ok(access_token) = self.credential_service.acquire(&account).await else {
            return;
        };

        match publisher
            .recover_upload(&access_token, session.clone(), &video)
            .await
        {
            Ok(recovered) => {
                let _ = self.session_repo.update(&recovered).await;
                match recovered.state {
                    RemoteUploadState::RemoteSucceeded => {
                        let _ = self
                            .publication_repo
                            .update_execution_state(
                                publication.id,
                                &claim_token,
                                &ExecutionStateUpdate {
                                    status: PublicationStatus::Published,
                                    remote_id: recovered.remote_publish_id.clone(),
                                    retry_count: publication.retry_count,
                                    last_error: None,
                                    last_error_code: None,
                                    rendered_metadata_json: None,
                                    release_claim: true,
                                    new_lease_expires_at: None,
                                    published_at: Some(Utc::now()),
                                },
                            )
                            .await;
                        self.log_success(&publication).await;
                    }
                    RemoteUploadState::RemoteProcessing | RemoteUploadState::Transferred => {
                        let _ = self
                            .publication_repo
                            .update_execution_state(
                                publication.id,
                                &claim_token,
                                &ExecutionStateUpdate {
                                    status: PublicationStatus::Processing,
                                    remote_id: recovered.remote_publish_id.clone(),
                                    retry_count: publication.retry_count,
                                    last_error: None,
                                    last_error_code: None,
                                    rendered_metadata_json: None,
                                    release_claim: true,
                                    new_lease_expires_at: None,
                                    published_at: None,
                                },
                            )
                            .await;
                    }
                    _ => {
                        let _ = self
                            .publication_repo
                            .update_execution_state(
                                publication.id,
                                &claim_token,
                                &ExecutionStateUpdate {
                                    status: PublicationStatus::Failed,
                                    remote_id: None,
                                    retry_count: publication.retry_count + 1,
                                    last_error: Some(
                                        "recovery reported a failed remote result".to_string(),
                                    ),
                                    last_error_code: None,
                                    rendered_metadata_json: None,
                                    release_claim: true,
                                    new_lease_expires_at: None,
                                    published_at: None,
                                },
                            )
                            .await;
                        self.requeue_for_retry(&publication, publication.retry_count + 1, None)
                            .await;
                    }
                }
            }
            Err(_) => {
                // Section 69: the outcome is still genuinely unknown —
                // fail closed rather than guessing. Releasing the claim
                // without restarting keeps this out of the due-scan
                // (status stays `Uploading` with no lease, so nothing
                // else picks it up) until a human verifies and retries
                // it manually.
                let _ = self
                    .publication_repo
                    .update_execution_state(
                        publication.id,
                        &claim_token,
                        &ExecutionStateUpdate {
                            status: PublicationStatus::Failed,
                            remote_id: None,
                            retry_count: publication.retry_count,
                            last_error: Some(PublishError::UnknownRemoteResult.user_message()),
                            last_error_code: Some(
                                PublishError::UnknownRemoteResult.code().to_string(),
                            ),
                            rendered_metadata_json: None,
                            release_claim: true,
                            new_lease_expires_at: None,
                            published_at: None,
                        },
                    )
                    .await;
            }
        }
    }

    async fn freeze_metadata(
        &self,
        publication_id: Uuid,
        claim_token: &str,
        metadata: &RenderedMetadata,
    ) {
        if let Ok(json) = serde_json::to_string(metadata) {
            let _ = self
                .publication_repo
                .update_execution_state(
                    publication_id,
                    claim_token,
                    &ExecutionStateUpdate {
                        status: PublicationStatus::Uploading,
                        remote_id: None,
                        retry_count: 0,
                        last_error: None,
                        last_error_code: None,
                        rendered_metadata_json: Some(json),
                        release_claim: false,
                        new_lease_expires_at: Some(Utc::now() + CLAIM_LEASE_DURATION),
                        published_at: None,
                    },
                )
                .await;
        }
    }

    async fn finish_attempt_failure(
        &self,
        publication: &Publication,
        claim_token: &str,
        attempt: &mut PublicationAttempt,
        err: PublishError,
    ) -> Result<(), PublishError> {
        attempt.fail(&err);
        let _ = self.attempt_repo.update(attempt).await;
        self.fail_and_release(publication, claim_token, err, attempt.attempt_number)
            .await
    }

    async fn fail_and_release(
        &self,
        publication: &Publication,
        claim_token: &str,
        err: PublishError,
        attempt_number: i32,
    ) -> Result<(), PublishError> {
        // Section 18/19: the engine is the one authoritative place a
        // provider's rate-limit signal gets persisted — never the
        // uploader itself.
        if let (
            PublishError::RateLimited {
                retry_after_seconds,
            },
            Some(account_id),
        ) = (&err, publication.platform_account_id)
        {
            let _ = self
                .rate_limit_service
                .record_rate_limited(
                    account_id,
                    RateLimitOperation::Publish,
                    *retry_after_seconds,
                )
                .await;
        }
        let retryable = err.is_retryable() && !attempts_exhausted(attempt_number);
        let released = self
            .publication_repo
            .update_execution_state(
                publication.id,
                claim_token,
                &ExecutionStateUpdate {
                    status: PublicationStatus::Failed,
                    remote_id: None,
                    retry_count: publication.retry_count + 1,
                    last_error: Some(err.user_message()),
                    last_error_code: Some(err.code().to_string()),
                    rendered_metadata_json: None,
                    release_claim: true,
                    new_lease_expires_at: None,
                    published_at: None,
                },
            )
            .await
            .unwrap_or(false);

        if released && retryable {
            self.requeue_for_retry(
                publication,
                publication.retry_count + 1,
                err.retry_after_seconds(),
            )
            .await;
        } else if released {
            let _ = self
                .notification_service
                .notify(
                    crate::domain::notification::NotificationType::Error,
                    format!("{} publish failed", publication.platform.display_name()),
                    format!(
                        "\"{}\" couldn't be published: {}",
                        publication.title,
                        err.user_message()
                    ),
                )
                .await;
        }
        Err(err)
    }

    /// Moves a `Failed` publication back through the valid state-machine
    /// chain (`Failed -> RetryWait -> Queued -> Scheduled`) with a
    /// jittered backoff delay, so the next due-scan picks it up again —
    /// this is a plain `update()` call, not `update_execution_state`,
    /// because by the time this runs the claim has already been released
    /// and the row is no longer `Uploading`/`Processing` (section 73).
    async fn requeue_for_retry(
        &self,
        publication: &Publication,
        attempt_number: i32,
        provider_retry_after_seconds: Option<u64>,
    ) {
        let Ok(Some(mut fresh)) = self.publication_repo.get(publication.id).await else {
            return;
        };
        if fresh.status != PublicationStatus::Failed {
            return;
        }
        let retry_at = next_retry_at(
            attempt_number.max(1),
            Utc::now(),
            provider_retry_after_seconds,
        );
        if fresh.transition(PublicationStatus::RetryWait).is_err() {
            return;
        }
        if fresh.transition(PublicationStatus::Queued).is_err() {
            return;
        }
        if fresh.transition(PublicationStatus::Scheduled).is_err() {
            return;
        }
        fresh.scheduled_at = Some(retry_at);
        let _ = self.publication_repo.update(&fresh).await;
    }

    /// Section 57: the `Skip` missed-schedule policy for a publication
    /// overdue beyond its grace period. `Scheduled` has no direct
    /// transition to `Failed` in the state machine, so this reuses the
    /// existing `Cancelled` terminal state rather than adding a new one
    /// — the distinguishing signal from a manual cancel is `last_error`
    /// and this specific activity-log entry.
    async fn skip_missed_publication(&self, publication: &Publication) {
        let Ok(Some(mut fresh)) = self.publication_repo.get(publication.id).await else {
            return;
        };
        if fresh.status != PublicationStatus::Scheduled {
            return;
        }
        if fresh.transition(PublicationStatus::Cancelled).is_err() {
            return;
        }
        fresh.last_error = Some(
            "Missed its scheduled window beyond the configured grace period; the workspace's missed-schedule policy is set to skip execution".to_string(),
        );
        if self.publication_repo.update(&fresh).await.is_ok() {
            let _ = self
                .activity_service
                .log(
                    ActivityCategory::Publication,
                    ActivityLevel::Warning,
                    format!(
                        "\"{}\" was skipped — it missed its scheduled window",
                        publication.title
                    ),
                )
                .await;
        }
    }

    async fn log_success(&self, publication: &Publication) {
        let _ = self
            .activity_service
            .log(
                ActivityCategory::Publication,
                ActivityLevel::Success,
                format!(
                    "\"{}\" published to {}",
                    publication.title, publication.platform
                ),
            )
            .await;
        let _ = self
            .notification_service
            .notify(
                crate::domain::notification::NotificationType::Success,
                format!("{} published", publication.platform.display_name()),
                format!("\"{}\" is now live.", publication.title),
            )
            .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    use crate::application::activity_service::ActivityService;
    use crate::application::platform_auth_service::PlatformAuthService;
    use crate::domain::auth_error::AuthError;
    use crate::domain::capability::Capability;
    use crate::domain::platform_account::{PlatformAccount, PlatformAccountStatus};
    use crate::domain::ports::platform_connector::PlatformConnector;
    use crate::domain::provider_identity::{ConnectedIdentity, RefreshedCredentials};
    use crate::domain::publication::PublicationStatus;
    use crate::domain::video::Video;
    use crate::infrastructure::hashing::content_hash::Sha256ContentHashService;
    use crate::infrastructure::publishing::{FakePublisher, FakeScenario};
    use crate::infrastructure::repositories::{
        SqliteActivityRepository, SqliteChannelRepository, SqliteNotificationRepository,
        SqlitePlatformAccountRepository, SqlitePublicationAttemptRepository,
        SqlitePublicationConsentRepository, SqlitePublicationRepository, SqliteSettingsRepository,
        SqliteUploadSessionRepository, SqliteVideoRepository,
    };
    use crate::test_support::*;

    /// Always returns a fixed token — the tests below exercise the
    /// engine's own exactly-once/recovery logic, not credential
    /// acquisition, so this stays deliberately trivial.
    struct FakeTokenConnector;

    #[async_trait]
    impl PlatformConnector for FakeTokenConnector {
        fn platform(&self) -> Platform {
            Platform::TikTok
        }
        async fn validate_connection(
            &self,
            _account: &PlatformAccount,
        ) -> Result<ConnectedIdentity, AuthError> {
            unimplemented!()
        }
        async fn refresh_connection(
            &self,
            _account: &PlatformAccount,
        ) -> Result<RefreshedCredentials, AuthError> {
            unimplemented!()
        }
        async fn disconnect(&self, _account: &PlatformAccount) -> Result<(), AuthError> {
            Ok(())
        }
        async fn get_profile(
            &self,
            _account: &PlatformAccount,
        ) -> Result<ConnectedIdentity, AuthError> {
            unimplemented!()
        }
        async fn acquire_access_token(
            &self,
            _account: &PlatformAccount,
        ) -> Result<String, AuthError> {
            Ok("fake-access-token".to_string())
        }
    }

    struct TestFixture {
        pool: sqlx::SqlitePool,
        engine: PublishingEngineService,
        publication_repo: Arc<SqlitePublicationRepository>,
        attempt_repo: Arc<SqlitePublicationAttemptRepository>,
        session_repo: Arc<SqliteUploadSessionRepository>,
        workspace_id: Uuid,
        channel_id: Uuid,
        source_id: Uuid,
    }

    async fn build_fixture(scenario: FakeScenario) -> TestFixture {
        let pool = temp_pool("publishing-engine").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Channel").await;

        let publication_repo = Arc::new(SqlitePublicationRepository::new(pool.clone()));
        let attempt_repo = Arc::new(SqlitePublicationAttemptRepository::new(pool.clone()));
        let session_repo = Arc::new(SqliteUploadSessionRepository::new(pool.clone()));
        let video_repo = Arc::new(SqliteVideoRepository::new(pool.clone()));
        let platform_account_repo = Arc::new(SqlitePlatformAccountRepository::new(pool.clone()));
        let channel_repo = Arc::new(SqliteChannelRepository::new(pool.clone()));
        let activity_service = Arc::new(ActivityService::new(Arc::new(
            SqliteActivityRepository::new(pool.clone()),
        )));
        let notification_service = Arc::new(NotificationService::new(Arc::new(
            SqliteNotificationRepository::new(pool.clone()),
        )));

        let platform_auth_service = Arc::new(PlatformAuthService::new(
            HashMap::new(),
            HashMap::new(),
            platform_account_repo.clone(),
            activity_service.clone(),
            notification_service.clone(),
        ));
        let mut token_connectors: HashMap<Platform, Arc<dyn PlatformConnector>> = HashMap::new();
        token_connectors.insert(Platform::TikTok, Arc::new(FakeTokenConnector));
        let credential_service = Arc::new(CredentialAcquisitionService::new(
            token_connectors,
            platform_auth_service,
        ));

        let mut publishers: HashMap<Platform, Arc<dyn PlatformPublisher>> = HashMap::new();
        publishers.insert(
            Platform::TikTok,
            Arc::new(FakePublisher::new(Platform::TikTok, scenario)),
        );

        let metadata_service = Arc::new(
            crate::application::metadata_template_service::MetadataTemplateService::new(
                Arc::new(
                    crate::infrastructure::repositories::SqliteMetadataTemplateRepository::new(
                        pool.clone(),
                    ),
                ),
                Arc::new(
                    crate::infrastructure::repositories::SqliteHashtagSetRepository::new(
                        pool.clone(),
                    ),
                ),
                publication_repo.clone() as Arc<dyn PublicationRepository>,
                video_repo.clone() as Arc<dyn VideoRepository>,
                Arc::new(
                    crate::infrastructure::repositories::SqliteVideoSourceRepository::new(
                        pool.clone(),
                    ),
                )
                    as Arc<dyn crate::domain::ports::repositories::VideoSourceRepository>,
                channel_repo.clone() as Arc<dyn ChannelRepository>,
                publishers.clone(),
            ),
        );

        let engine = PublishingEngineService::new(
            publication_repo.clone() as Arc<dyn PublicationRepository>,
            attempt_repo.clone() as Arc<dyn PublicationAttemptRepository>,
            session_repo.clone() as Arc<dyn UploadSessionRepository>,
            video_repo.clone() as Arc<dyn VideoRepository>,
            platform_account_repo.clone() as Arc<dyn PlatformAccountRepository>,
            channel_repo as Arc<dyn ChannelRepository>,
            Arc::new(SqlitePublicationConsentRepository::new(pool.clone()))
                as Arc<dyn PublicationConsentRepository>,
            Arc::new(Sha256ContentHashService),
            credential_service,
            metadata_service,
            Arc::new(
                crate::application::provider_rate_limit_service::ProviderRateLimitService::new(
                    Arc::new(
                        crate::infrastructure::repositories::SqliteProviderRateStateRepository::new(
                            pool.clone(),
                        ),
                    ),
                ),
            ),
            Arc::new(SettingsService::new(Arc::new(
                SqliteSettingsRepository::new(pool.clone()),
            ))),
            Arc::new(crate::domain::ports::progress_publisher::NullProgressPublisher),
            publishers,
            activity_service,
            notification_service,
        );

        TestFixture {
            pool,
            engine,
            publication_repo,
            attempt_repo,
            session_repo,
            workspace_id,
            channel_id,
            source_id,
        }
    }

    /// Seeds a real video file on disk (the engine checks `path.exists()`
    /// before ever touching the network — section 118), a `Connected`
    /// account with `UploadVideo`, and a due `Scheduled` publication
    /// targeting it. Returns the publication id.
    async fn seed_ready_publication(
        pool: &sqlx::SqlitePool,
        workspace_id: Uuid,
        channel_id: Uuid,
        source_id: Uuid,
    ) -> Uuid {
        let dir = temp_dir("publishing-engine-video");
        let path = write_fake_video(&dir, "clip.mp4", b"fake video bytes");

        let video = Video::new(
            workspace_id,
            source_id,
            Some(channel_id),
            "clip.mp4",
            "Clip",
            path.display().to_string(),
            path.metadata().unwrap().len() as i64,
            "mp4",
        );
        let video_repo = SqliteVideoRepository::new(pool.clone());
        use crate::domain::ports::repositories::VideoRepository;
        video_repo.create(&video).await.unwrap();

        let mut account = PlatformAccount::new(workspace_id, channel_id, Platform::TikTok);
        account.status = PlatformAccountStatus::Connected;
        account.provider_connection_id = Some("conn-1".to_string());
        account.capabilities = vec![Capability::ReadProfile, Capability::UploadVideo];
        let account_repo = SqlitePlatformAccountRepository::new(pool.clone());
        use crate::domain::ports::repositories::PlatformAccountRepository;
        account_repo.create(&account).await.unwrap();

        let mut publication = Publication::new(
            workspace_id,
            video.id,
            channel_id,
            Platform::TikTok,
            "Great clip",
            crate::domain::video_status::VideoPriority::Normal,
        );
        publication.platform_account_id = Some(account.id);
        publication.status = PublicationStatus::Scheduled;
        publication.scheduled_at = Some(Utc::now() - Duration::minutes(1));
        let publication_repo = SqlitePublicationRepository::new(pool.clone());
        publication_repo.create(&publication).await.unwrap();

        // The fixtures use TikTok throughout, which requires express
        // consent (section 31) — seed a matching approval up front so
        // tests that aren't specifically about consent don't have to
        // think about it. `metadata_consent_hash_for` mirrors exactly
        // what `execute_inner` renders from a freshly created
        // publication, so it must be kept in sync if that rendering ever
        // changes.
        let consent_repo = SqlitePublicationConsentRepository::new(pool.clone());
        let consent = crate::domain::publishing::PublicationConsent::new(
            publication.id,
            Platform::TikTok,
            metadata_consent_hash_for(&publication),
            crate::domain::publishing::ApprovalSource::AddToQueue,
        );
        consent_repo.create(&consent).await.unwrap();

        publication.id
    }

    fn metadata_consent_hash_for(publication: &Publication) -> String {
        RenderedMetadata {
            title: publication.title.clone(),
            description: publication.description.clone().unwrap_or_default(),
            hashtags: publication.hashtags.clone(),
            provider_options: serde_json::json!({}),
        }
        .consent_hash()
    }

    #[tokio::test]
    async fn a_successful_publish_reaches_published_with_one_succeeded_attempt() {
        let fixture = build_fixture(FakeScenario::Success).await;
        let pool = fixture.pool.clone();
        let publication_id = seed_ready_publication(
            &pool,
            fixture.workspace_id,
            fixture.channel_id,
            fixture.source_id,
        )
        .await;

        let claimed = fixture
            .engine
            .scan_and_claim_due(fixture.workspace_id)
            .await;
        assert_eq!(claimed.len(), 1);
        let (id, claim_token) = claimed.into_iter().next().unwrap();
        assert_eq!(id, publication_id);

        fixture.engine.execute(id, claim_token).await;

        let publication = fixture.publication_repo.get(id).await.unwrap().unwrap();
        assert_eq!(publication.status, PublicationStatus::Published);
        assert!(publication.published_at.is_some());
        assert!(
            publication.claim_token.is_none(),
            "the claim must be released"
        );
        assert!(
            publication.rendered_metadata.is_some(),
            "metadata must be frozen"
        );

        let attempts = fixture.attempt_repo.list_for_publication(id).await.unwrap();
        assert_eq!(attempts.len(), 1);
        assert_eq!(
            attempts[0].status,
            crate::domain::publishing::AttemptStatus::Succeeded
        );
    }

    #[tokio::test]
    async fn scan_and_claim_due_is_exactly_once_under_concurrency() {
        let fixture = build_fixture(FakeScenario::Success).await;
        let pool = fixture.pool.clone();
        seed_ready_publication(
            &pool,
            fixture.workspace_id,
            fixture.channel_id,
            fixture.source_id,
        )
        .await;

        let engine = Arc::new(fixture.engine);
        let workspace_id = fixture.workspace_id;
        let (a, b) = tokio::join!(
            {
                let engine = engine.clone();
                async move { engine.scan_and_claim_due(workspace_id).await }
            },
            {
                let engine = engine.clone();
                async move { engine.scan_and_claim_due(workspace_id).await }
            }
        );
        let total_claimed = a.len() + b.len();
        assert_eq!(
            total_claimed, 1,
            "the same publication must never be claimed twice"
        );
    }

    #[tokio::test]
    async fn a_permission_missing_account_fails_without_ever_touching_the_provider() {
        let fixture = build_fixture(FakeScenario::Success).await;
        let pool = fixture.pool.clone();
        let publication_id = seed_ready_publication(
            &pool,
            fixture.workspace_id,
            fixture.channel_id,
            fixture.source_id,
        )
        .await;

        // Strip the publish capability after seeding.
        let account_repo = SqlitePlatformAccountRepository::new(pool.clone());
        use crate::domain::ports::repositories::PlatformAccountRepository as _;
        let publication = fixture
            .publication_repo
            .get(publication_id)
            .await
            .unwrap()
            .unwrap();
        let mut account = account_repo
            .get(publication.platform_account_id.unwrap())
            .await
            .unwrap()
            .unwrap();
        account.capabilities = vec![Capability::ReadProfile];
        account_repo.update(&account).await.unwrap();

        let claimed = fixture
            .engine
            .scan_and_claim_due(fixture.workspace_id)
            .await;
        let (id, claim_token) = claimed.into_iter().next().unwrap();
        fixture.engine.execute(id, claim_token).await;

        let attempts = fixture.attempt_repo.list_for_publication(id).await.unwrap();
        assert!(
            attempts.is_empty(),
            "no attempt should be created for a permission check that fails pre-flight"
        );
    }

    #[tokio::test]
    async fn an_interrupted_upload_is_recovered_and_completes_exactly_once() {
        let fixture = build_fixture(FakeScenario::InterruptedThenRecoverable).await;
        let pool = fixture.pool.clone();
        let _publication_id = seed_ready_publication(
            &pool,
            fixture.workspace_id,
            fixture.channel_id,
            fixture.source_id,
        )
        .await;

        let claimed = fixture
            .engine
            .scan_and_claim_due(fixture.workspace_id)
            .await;
        let (id, claim_token) = claimed.into_iter().next().unwrap();
        fixture.engine.execute(id, claim_token).await;

        // The simulated network drop leaves it Failed (non-retryable path
        // skipped here — this test cares about the *session* recovery,
        // not the retry chain), then requeued back to Scheduled by the
        // engine's own retry logic. Force it back into an abandoned
        // Uploading claim with an expired lease to simulate "the process
        // was killed mid-upload" instead.
        let mut publication = fixture.publication_repo.get(id).await.unwrap().unwrap();
        publication.status = PublicationStatus::Scheduled;
        publication.scheduled_at = Some(Utc::now() - Duration::minutes(1));
        fixture.publication_repo.update(&publication).await.unwrap();

        let claimed_again = fixture
            .engine
            .scan_and_claim_due(fixture.workspace_id)
            .await;
        let (id2, claim_token2) = claimed_again.into_iter().next().unwrap();
        assert_eq!(id2, id);
        fixture.engine.execute(id2, claim_token2).await;

        // At this point the FIRST upload_media call already consumed the
        // one-shot "interrupt" branch (bytes_committed was 0 the first
        // time only), so this second attempt actually completes
        // normally. To exercise real crash recovery, manually re-arm an
        // interrupted session and an expired lease, then call
        // `recover_interrupted` directly.
        let session = fixture
            .session_repo
            .latest_for_publication(id)
            .await
            .unwrap();
        if let Some(mut session) = session {
            session.state = crate::domain::publishing::RemoteUploadState::Transferring;
            session.bytes_committed = 5;
            fixture.session_repo.update(&session).await.unwrap();
        }
        let claim_token3 = "manually-expired-claim".to_string();
        fixture
            .publication_repo
            .try_claim_due(
                id,
                Uuid::new_v4(),
                &claim_token3,
                Utc::now() - Duration::minutes(1),
                Utc::now() - Duration::hours(1),
            )
            .await
            .ok();

        fixture
            .engine
            .recover_interrupted(fixture.workspace_id)
            .await;

        // Whatever the final state, there must never be more than one
        // Published outcome's worth of attempts marked Succeeded.
        let attempts = fixture.attempt_repo.list_for_publication(id).await.unwrap();
        let succeeded = attempts
            .iter()
            .filter(|a| a.status == crate::domain::publishing::AttemptStatus::Succeeded)
            .count();
        assert!(
            succeeded <= 1,
            "at most one attempt may ever be marked succeeded"
        );
    }

    #[tokio::test]
    async fn processing_publications_only_finish_after_a_poll_confirms_success() {
        let fixture = build_fixture(FakeScenario::NeedsProcessing {
            polls_until_done: 2,
        })
        .await;
        let pool = fixture.pool.clone();
        let _publication_id = seed_ready_publication(
            &pool,
            fixture.workspace_id,
            fixture.channel_id,
            fixture.source_id,
        )
        .await;

        let claimed = fixture
            .engine
            .scan_and_claim_due(fixture.workspace_id)
            .await;
        let (id, claim_token) = claimed.into_iter().next().unwrap();
        fixture.engine.execute(id, claim_token).await;

        let after_upload = fixture.publication_repo.get(id).await.unwrap().unwrap();
        assert_eq!(after_upload.status, PublicationStatus::Processing);

        fixture.engine.poll_processing(id).await.unwrap();
        let after_first_poll = fixture.publication_repo.get(id).await.unwrap().unwrap();
        assert_eq!(
            after_first_poll.status,
            PublicationStatus::Processing,
            "not done after the first poll"
        );

        fixture.engine.poll_processing(id).await.unwrap();
        let after_second_poll = fixture.publication_repo.get(id).await.unwrap().unwrap();
        assert_eq!(after_second_poll.status, PublicationStatus::Published);
    }

    #[tokio::test]
    async fn a_paused_channel_is_never_claimed() {
        let fixture = build_fixture(FakeScenario::Success).await;
        let pool = fixture.pool.clone();
        seed_ready_publication(
            &pool,
            fixture.workspace_id,
            fixture.channel_id,
            fixture.source_id,
        )
        .await;

        sqlx::query("UPDATE channels SET status = 'paused' WHERE id = ?")
            .bind(fixture.channel_id.to_string())
            .execute(&pool)
            .await
            .unwrap();

        let claimed = fixture
            .engine
            .scan_and_claim_due(fixture.workspace_id)
            .await;
        assert!(claimed.is_empty());
    }

    #[tokio::test]
    async fn tiktok_without_a_matching_consent_record_never_reaches_the_provider() {
        let fixture = build_fixture(FakeScenario::Success).await;
        let pool = fixture.pool.clone();
        let publication_id = seed_ready_publication(
            &pool,
            fixture.workspace_id,
            fixture.channel_id,
            fixture.source_id,
        )
        .await;
        // seed_ready_publication seeds a matching consent by default —
        // remove it to exercise the "never approved" path.
        sqlx::query("DELETE FROM publication_consent WHERE publication_id = ?")
            .bind(publication_id.to_string())
            .execute(&pool)
            .await
            .unwrap();

        let claimed = fixture
            .engine
            .scan_and_claim_due(fixture.workspace_id)
            .await;
        let (id, claim_token) = claimed.into_iter().next().unwrap();
        fixture.engine.execute(id, claim_token).await;

        let publication = fixture.publication_repo.get(id).await.unwrap().unwrap();
        assert_eq!(publication.status, PublicationStatus::Failed);
        assert_eq!(
            publication.last_error.as_deref(),
            Some(PublishError::ConsentRequired.user_message().as_str())
        );
        let attempts = fixture.attempt_repo.list_for_publication(id).await.unwrap();
        assert!(
            attempts.is_empty(),
            "no attempt should be created without consent"
        );
    }

    #[tokio::test]
    async fn tiktok_consent_stops_covering_a_publication_once_its_title_changes() {
        let fixture = build_fixture(FakeScenario::Success).await;
        let pool = fixture.pool.clone();
        let publication_id = seed_ready_publication(
            &pool,
            fixture.workspace_id,
            fixture.channel_id,
            fixture.source_id,
        )
        .await;

        // Simulate an edit after approval (section 33): the stored
        // consent hash no longer matches the (now different) title.
        sqlx::query("UPDATE publications SET title = 'Edited after approval' WHERE id = ?")
            .bind(publication_id.to_string())
            .execute(&pool)
            .await
            .unwrap();

        let claimed = fixture
            .engine
            .scan_and_claim_due(fixture.workspace_id)
            .await;
        let (id, claim_token) = claimed.into_iter().next().unwrap();
        fixture.engine.execute(id, claim_token).await;

        let publication = fixture.publication_repo.get(id).await.unwrap().unwrap();
        assert_eq!(publication.status, PublicationStatus::Failed);
        assert_eq!(
            publication.last_error.as_deref(),
            Some(PublishError::ConsentRequired.user_message().as_str())
        );
    }

    #[tokio::test]
    async fn publish_now_executes_a_publication_scheduled_far_in_the_future() {
        let fixture = build_fixture(FakeScenario::Success).await;
        let pool = fixture.pool.clone();
        let publication_id = seed_ready_publication(
            &pool,
            fixture.workspace_id,
            fixture.channel_id,
            fixture.source_id,
        )
        .await;

        let mut publication = fixture
            .publication_repo
            .get(publication_id)
            .await
            .unwrap()
            .unwrap();
        publication.scheduled_at = Some(Utc::now() + Duration::hours(6));
        fixture.publication_repo.update(&publication).await.unwrap();

        // Not due yet — the periodic scan must not touch it.
        let claimed = fixture
            .engine
            .scan_and_claim_due(fixture.workspace_id)
            .await;
        assert!(claimed.is_empty());

        fixture.engine.publish_now(publication_id).await.unwrap();

        let published = fixture
            .publication_repo
            .get(publication_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(published.status, PublicationStatus::Published);
    }

    #[tokio::test]
    async fn publish_now_refuses_a_locked_publication() {
        let fixture = build_fixture(FakeScenario::Success).await;
        let pool = fixture.pool.clone();
        let publication_id = seed_ready_publication(
            &pool,
            fixture.workspace_id,
            fixture.channel_id,
            fixture.source_id,
        )
        .await;

        let mut publication = fixture
            .publication_repo
            .get(publication_id)
            .await
            .unwrap()
            .unwrap();
        publication.locked = true;
        fixture.publication_repo.update(&publication).await.unwrap();

        let result = fixture.engine.publish_now(publication_id).await;
        assert!(result.is_err());
        let unchanged = fixture
            .publication_repo
            .get(publication_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(unchanged.status, PublicationStatus::Scheduled);
    }

    #[tokio::test]
    async fn get_attempts_returns_the_recorded_attempt_history() {
        let fixture = build_fixture(FakeScenario::Success).await;
        let pool = fixture.pool.clone();
        let publication_id = seed_ready_publication(
            &pool,
            fixture.workspace_id,
            fixture.channel_id,
            fixture.source_id,
        )
        .await;

        let claimed = fixture
            .engine
            .scan_and_claim_due(fixture.workspace_id)
            .await;
        let (id, claim_token) = claimed.into_iter().next().unwrap();
        fixture.engine.execute(id, claim_token).await;

        let attempts = fixture.engine.get_attempts(publication_id).await;
        assert_eq!(attempts.len(), 1);
        assert_eq!(
            attempts[0].status,
            crate::domain::publishing::AttemptStatus::Succeeded
        );
    }

    #[tokio::test]
    async fn record_consent_unblocks_a_tiktok_publication_that_had_none() {
        let fixture = build_fixture(FakeScenario::Success).await;
        let pool = fixture.pool.clone();
        let publication_id = seed_ready_publication(
            &pool,
            fixture.workspace_id,
            fixture.channel_id,
            fixture.source_id,
        )
        .await;
        sqlx::query("DELETE FROM publication_consent WHERE publication_id = ?")
            .bind(publication_id.to_string())
            .execute(&pool)
            .await
            .unwrap();

        fixture
            .engine
            .record_consent(
                publication_id,
                crate::domain::publishing::ApprovalSource::ManualSchedule,
            )
            .await
            .unwrap();

        let claimed = fixture
            .engine
            .scan_and_claim_due(fixture.workspace_id)
            .await;
        let (id, claim_token) = claimed.into_iter().next().unwrap();
        fixture.engine.execute(id, claim_token).await;

        let publication = fixture.publication_repo.get(id).await.unwrap().unwrap();
        assert_eq!(publication.status, PublicationStatus::Published);
    }

    #[tokio::test]
    async fn a_rate_limited_account_is_never_dispatched_to_the_provider() {
        let fixture = build_fixture(FakeScenario::Success).await;
        let pool = fixture.pool.clone();
        let publication_id = seed_ready_publication(
            &pool,
            fixture.workspace_id,
            fixture.channel_id,
            fixture.source_id,
        )
        .await;
        let publication = fixture
            .publication_repo
            .get(publication_id)
            .await
            .unwrap()
            .unwrap();
        let account_id = publication.platform_account_id.unwrap();

        let rate_repo = crate::infrastructure::repositories::SqliteProviderRateStateRepository::new(
            pool.clone(),
        );
        use crate::domain::ports::repositories::ProviderRateStateRepository;
        rate_repo
            .record_rate_limit(account_id, "publish", Utc::now() + Duration::minutes(30))
            .await
            .unwrap();

        let claimed = fixture
            .engine
            .scan_and_claim_due(fixture.workspace_id)
            .await;
        let (id, claim_token) = claimed.into_iter().next().unwrap();
        fixture.engine.execute(id, claim_token).await;

        let updated = fixture.publication_repo.get(id).await.unwrap().unwrap();
        assert_eq!(updated.status, PublicationStatus::Scheduled);
        assert!(
            updated
                .scheduled_at
                .is_some_and(|at| at > Utc::now() + Duration::minutes(25)),
            "the retry must be scheduled for after the known rate-limit window clears"
        );

        let attempts = fixture.attempt_repo.list_for_publication(id).await.unwrap();
        assert!(
            attempts.is_empty(),
            "a rate-limited account must never even reach attempt creation, let alone the provider"
        );
    }

    #[tokio::test]
    async fn a_global_pause_stops_the_scan_from_claiming_anything() {
        let fixture = build_fixture(FakeScenario::Success).await;
        let pool = fixture.pool.clone();
        seed_ready_publication(
            &pool,
            fixture.workspace_id,
            fixture.channel_id,
            fixture.source_id,
        )
        .await;

        let settings_service =
            SettingsService::new(Arc::new(SqliteSettingsRepository::new(pool.clone())));
        settings_service.set_publishing_paused(true).await.unwrap();

        let claimed = fixture
            .engine
            .scan_and_claim_due(fixture.workspace_id)
            .await;
        assert!(claimed.is_empty(), "a paused workspace must claim nothing");

        // publish_now must not be a backdoor around the pause either.
        let publication_id = fixture
            .publication_repo
            .list_due(fixture.workspace_id, Utc::now())
            .await
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .id;
        let result = fixture.engine.publish_now(publication_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn the_skip_missed_schedule_policy_cancels_an_overdue_publication_instead_of_publishing_it(
    ) {
        let fixture = build_fixture(FakeScenario::Success).await;
        let pool = fixture.pool.clone();
        let publication_id = seed_ready_publication(
            &pool,
            fixture.workspace_id,
            fixture.channel_id,
            fixture.source_id,
        )
        .await;
        let mut publication = fixture
            .publication_repo
            .get(publication_id)
            .await
            .unwrap()
            .unwrap();
        publication.scheduled_at = Some(Utc::now() - Duration::hours(2));
        fixture.publication_repo.update(&publication).await.unwrap();

        let settings_service =
            SettingsService::new(Arc::new(SqliteSettingsRepository::new(pool.clone())));
        settings_service
            .update_publishing(
                crate::application::settings_service::UpdatePublishingSettingsInput {
                    missed_schedule_policy: Some(
                        crate::domain::app_settings::MissedSchedulePolicy::Skip,
                    ),
                    missed_schedule_grace_period_minutes: Some(0),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        let claimed = fixture
            .engine
            .scan_and_claim_due(fixture.workspace_id)
            .await;
        assert!(claimed.is_empty());

        let updated = fixture
            .publication_repo
            .get(publication_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, PublicationStatus::Cancelled);
        assert!(updated.last_error.is_some());
    }

    #[tokio::test]
    async fn an_ambiguous_recovery_result_fails_closed_with_a_stable_error_code_never_auto_retried()
    {
        let fixture = build_fixture(FakeScenario::AmbiguousAfterCrash).await;
        let pool = fixture.pool.clone();
        let publication_id = seed_ready_publication(
            &pool,
            fixture.workspace_id,
            fixture.channel_id,
            fixture.source_id,
        )
        .await;

        // Claim it with an already-expired lease, as if the process died
        // mid-upload, and leave a session in a not-safe-to-restart state
        // — exactly the precondition `recover_one` inspects.
        let claim_token = "expired-claim".to_string();
        let claimed = fixture
            .publication_repo
            .try_claim_due(
                publication_id,
                Uuid::new_v4(),
                &claim_token,
                Utc::now() - Duration::minutes(1),
                Utc::now(),
            )
            .await
            .unwrap();
        assert!(claimed, "the setup claim must actually succeed");
        let attempt =
            crate::domain::publishing::PublicationAttempt::new(publication_id, 1, Platform::TikTok);
        fixture.attempt_repo.create(&attempt).await.unwrap();
        let mut session = crate::domain::publishing::UploadSession::new(
            publication_id,
            attempt.id,
            Platform::TikTok,
            crate::domain::publishing::SessionType::DirectPost,
        );
        session.state = crate::domain::publishing::RemoteUploadState::Transferring;
        session.bytes_committed = 5;
        fixture.session_repo.create(&session).await.unwrap();

        fixture
            .engine
            .recover_interrupted(fixture.workspace_id)
            .await;

        let updated = fixture
            .publication_repo
            .get(publication_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            updated.status,
            PublicationStatus::Failed,
            "an ambiguous recovery must fail closed, not silently succeed or restart"
        );
        assert_eq!(
            updated.last_error_code.as_deref(),
            Some("UNKNOWN_REMOTE_RESULT"),
            "the frontend needs this stable code to refuse blind retry — free-text alone isn't enough"
        );
        assert!(
            updated.claim_token.is_none(),
            "the claim must still be released so it doesn't look abandoned forever"
        );
        // Section 69: this must NOT have been silently requeued for
        // automatic retry — it stays exactly where a human left it,
        // still Failed, not bounced back to Scheduled.
        assert!(updated.scheduled_at.is_none() || updated.status == PublicationStatus::Failed);
    }
}
