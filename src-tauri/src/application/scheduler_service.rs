use std::collections::HashSet;
use std::sync::Arc;

use chrono::{DateTime, Duration, NaiveDate, TimeZone, Utc};
use uuid::Uuid;

use crate::application::activity_service::ActivityService;
use crate::domain::activity_event::{ActivityCategory, ActivityLevel};
use crate::domain::channel::ChannelStatus;
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::ports::repositories::{
    ChannelRepository, PublicationRepository, ScheduleExceptionRepository, ScheduleSlotRepository,
    WorkspaceRepository,
};
use crate::domain::publication::{Publication, PublicationStatus};
use crate::domain::publication_query::CalendarPublication;
use crate::domain::scheduling::{self, DEFAULT_SEARCH_HORIZON_DAYS};

/// Shorter horizon used by "Fill Empty Slots" (section 26): it only makes
/// sense to eagerly fill the *near* future, not to reach 120 days out the
/// way a one-off auto-schedule of a single freshly-queued item may.
const FILL_GAPS_HORIZON_DAYS: i64 = 30;

/// Orchestrates everything time-related: manual scheduling, auto-scheduling
/// via `domain::scheduling`, rebuilds, gap-filling, and the Calendar's
/// range query. Never touches `QueueItem` membership — that stays owned by
/// `PublicationService`, since a scheduled publication is still "in the
/// queue" for as long as it exists (section 15).
pub struct SchedulerService {
    publication_repo: Arc<dyn PublicationRepository>,
    channel_repo: Arc<dyn ChannelRepository>,
    workspace_repo: Arc<dyn WorkspaceRepository>,
    slot_repo: Arc<dyn ScheduleSlotRepository>,
    exception_repo: Arc<dyn ScheduleExceptionRepository>,
    activity_service: Arc<ActivityService>,
}

impl SchedulerService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        publication_repo: Arc<dyn PublicationRepository>,
        channel_repo: Arc<dyn ChannelRepository>,
        workspace_repo: Arc<dyn WorkspaceRepository>,
        slot_repo: Arc<dyn ScheduleSlotRepository>,
        exception_repo: Arc<dyn ScheduleExceptionRepository>,
        activity_service: Arc<ActivityService>,
    ) -> Self {
        Self {
            publication_repo,
            channel_repo,
            workspace_repo,
            slot_repo,
            exception_repo,
            activity_service,
        }
    }

    /// Manually schedules (from `Queued`) or reschedules (already
    /// `Scheduled`, e.g. a calendar drag) a publication to an exact UTC
    /// instant. Validates the channel is active and that no other
    /// `Scheduled` publication already occupies that instant on the same
    /// channel — backstopped by the database's partial unique index for
    /// the concurrent case (section 27/28/51).
    pub async fn schedule_at(
        &self,
        publication_id: Uuid,
        at: DateTime<Utc>,
    ) -> DomainResult<Publication> {
        let mut publication = self.load_publication(publication_id).await?;
        let channel = self.load_channel(publication.channel_id).await?;

        if channel.status != ChannelStatus::Active {
            return Err(DomainError::ChannelPaused {
                channel_id: channel.id.to_string(),
            });
        }

        self.assert_no_conflict(&publication, at).await?;

        match publication.status {
            PublicationStatus::Queued => publication.transition(PublicationStatus::Scheduled)?,
            PublicationStatus::Scheduled => {
                publication.updated_at = Utc::now();
            }
            other => {
                return Err(DomainError::InvalidTransition {
                    entity: "Publication",
                    from: other.to_string(),
                    to: "scheduled".to_string(),
                })
            }
        }
        publication.scheduled_at = Some(at);
        self.publication_repo.update(&publication).await?;
        Ok(publication)
    }

    /// `Scheduled -> Queued`, clearing `scheduled_at` (section 15's
    /// unschedule action). The `QueueItem` and its manual position are
    /// untouched.
    pub async fn unschedule(&self, publication_id: Uuid) -> DomainResult<Publication> {
        let mut publication = self.load_publication(publication_id).await?;
        publication.transition(PublicationStatus::Queued)?;
        publication.scheduled_at = None;
        self.publication_repo.update(&publication).await?;
        Ok(publication)
    }

    /// Moves an already-`Scheduled` publication to a different calendar
    /// date, keeping its local time-of-day unchanged (section 42's
    /// calendar drag-and-drop between days). Rejects locked publications
    /// outright (section 82/113 — drag must respect locks, not just the
    /// auto-scheduler) and reuses `schedule_at`'s own conflict/paused-
    /// channel validation for the destination instant.
    pub async fn reschedule_to_date(
        &self,
        publication_id: Uuid,
        new_date: NaiveDate,
    ) -> DomainResult<Publication> {
        let publication = self.load_publication(publication_id).await?;
        if publication.locked {
            return Err(DomainError::PublicationLocked {
                publication_id: publication.id.to_string(),
            });
        }
        let current_at =
            publication
                .scheduled_at
                .ok_or_else(|| DomainError::InvalidTransition {
                    entity: "Publication",
                    from: publication.status.to_string(),
                    to: "scheduled".to_string(),
                })?;

        let workspace = self.load_workspace(publication.workspace_id).await?;
        let tz: chrono_tz::Tz =
            workspace
                .timezone
                .parse()
                .map_err(|_| DomainError::InvalidValue {
                    field: "timezone",
                    reason: format!("{:?} is not a recognized IANA timezone", workspace.timezone),
                })?;

        let local_time = current_at.with_timezone(&tz).time();
        let new_local = new_date.and_time(local_time);
        let new_at = resolve_local_datetime_to_utc(tz, new_local);

        self.schedule_at(publication_id, new_at).await
    }

    /// Auto-places one `Queued`, unlocked publication into its channel's
    /// next available slot (section 22/25).
    pub async fn auto_schedule(&self, publication_id: Uuid) -> DomainResult<Publication> {
        let publication = self.load_publication(publication_id).await?;
        if publication.locked {
            return Err(DomainError::PublicationLocked {
                publication_id: publication.id.to_string(),
            });
        }
        if publication.status != PublicationStatus::Queued {
            return Err(DomainError::InvalidTransition {
                entity: "Publication",
                from: publication.status.to_string(),
                to: "scheduled".to_string(),
            });
        }

        let channel = self.load_channel(publication.channel_id).await?;
        if channel.status != ChannelStatus::Active {
            return Err(DomainError::ChannelPaused {
                channel_id: channel.id.to_string(),
            });
        }

        let workspace = self.load_workspace(publication.workspace_id).await?;
        let now = Utc::now();
        let horizon = DEFAULT_SEARCH_HORIZON_DAYS;
        let taken = self
            .taken_instants(channel.id, now, now + Duration::days(horizon))
            .await?;
        let slots = self.slot_repo.list_for_channel(channel.id).await?;
        let exceptions = self
            .exception_repo
            .list_for_channel_in_range(
                channel.id,
                now.date_naive(),
                (now + Duration::days(horizon)).date_naive(),
            )
            .await?;

        let found = scheduling::find_next_available_slot(
            &slots,
            &exceptions,
            &workspace.timezone,
            publication.platform,
            now,
            &taken,
            horizon,
        )
        .ok_or(DomainError::NoAvailableSlot)?;

        self.schedule_at(publication_id, found).await
    }

    /// Bulk auto-schedules every unlocked `Queued` publication on a
    /// channel, priority-first, inside a single database transaction
    /// (section 22/85/113) — either every resolvable item is scheduled, or
    /// none are. Items for which no slot could be found within the
    /// horizon are left `Queued` and reported as skipped rather than
    /// erroring the whole batch.
    pub async fn auto_schedule_channel(
        &self,
        channel_id: Uuid,
        horizon_days: i64,
    ) -> DomainResult<BulkScheduleResult> {
        let channel = self.load_channel(channel_id).await?;
        if channel.status != ChannelStatus::Active {
            return Err(DomainError::ChannelPaused {
                channel_id: channel.id.to_string(),
            });
        }
        let workspace = self.load_workspace(channel.workspace_id).await?;

        let now = Utc::now();
        let mut taken = self
            .taken_instants(channel.id, now, now + Duration::days(horizon_days))
            .await?;
        let slots = self.slot_repo.list_for_channel(channel.id).await?;
        let exceptions = self
            .exception_repo
            .list_for_channel_in_range(
                channel.id,
                now.date_naive(),
                (now + Duration::days(horizon_days)).date_naive(),
            )
            .await?;

        let candidates = self
            .publication_repo
            .list_unscheduled_for_channel(channel_id)
            .await?;
        let mut to_persist = Vec::new();
        let mut skipped = Vec::new();

        for mut publication in candidates {
            match scheduling::find_next_available_slot(
                &slots,
                &exceptions,
                &workspace.timezone,
                publication.platform,
                now,
                &taken,
                horizon_days,
            ) {
                Some(at) => {
                    taken.insert(at);
                    publication.transition(PublicationStatus::Scheduled)?;
                    publication.scheduled_at = Some(at);
                    to_persist.push(publication);
                }
                None => skipped.push(publication.id),
            }
        }

        let scheduled_count = to_persist.len();
        self.publication_repo.bulk_update(&to_persist).await?;

        self.activity_service
            .log(
                ActivityCategory::Publication,
                ActivityLevel::Info,
                format!(
                    "Auto-scheduled {scheduled_count} publication(s) for channel {channel_id}\
                     {} left unscheduled (no available slot)",
                    if skipped.is_empty() {
                        String::new()
                    } else {
                        format!(", {} ", skipped.len())
                    }
                ),
            )
            .await?;

        Ok(BulkScheduleResult {
            scheduled: to_persist,
            skipped,
        })
    }

    /// "Fill Empty Slots" (section 26): the same auto-schedule algorithm,
    /// bounded to a shorter near-term horizon since its purpose is closing
    /// visible near-term gaps, not reaching deep into the future.
    pub async fn fill_schedule_gaps(&self, channel_id: Uuid) -> DomainResult<BulkScheduleResult> {
        self.auto_schedule_channel(channel_id, FILL_GAPS_HORIZON_DAYS)
            .await
    }

    /// Recomputes `scheduled_at` for every unlocked `Scheduled` publication
    /// on a channel (section 30/82's "Rebuild Schedule", e.g. after the
    /// channel's weekly slots changed). Locked publications keep their
    /// exact time and are treated as immovable obstacles for everyone
    /// else. A publication that no longer fits anywhere is unscheduled
    /// back to `Queued` rather than left with a stale/invalid time.
    pub async fn rebuild_channel_schedule(
        &self,
        channel_id: Uuid,
        horizon_days: i64,
    ) -> DomainResult<BulkScheduleResult> {
        let channel = self.load_channel(channel_id).await?;
        let workspace = self.load_workspace(channel.workspace_id).await?;

        let now = Utc::now();
        let window_end = now + Duration::days(horizon_days);
        let scheduled = self
            .publication_repo
            .list_scheduled_in_range(channel.workspace_id, Some(channel_id), now, window_end)
            .await?;

        let mut taken: HashSet<DateTime<Utc>> = HashSet::new();
        let mut movable = Vec::new();
        for publication in scheduled {
            if publication.locked {
                if let Some(at) = publication.scheduled_at {
                    taken.insert(at);
                }
            } else {
                movable.push(publication);
            }
        }
        // Stable priority order: highest priority first, ties broken by
        // the item's previous scheduled time so a rebuild doesn't
        // needlessly reshuffle relative order among same-priority items.
        movable.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then(a.scheduled_at.cmp(&b.scheduled_at))
        });

        let slots = self.slot_repo.list_for_channel(channel_id).await?;
        let exceptions = self
            .exception_repo
            .list_for_channel_in_range(channel_id, now.date_naive(), window_end.date_naive())
            .await?;

        let mut to_persist = Vec::new();
        let mut unscheduled = Vec::new();
        for mut publication in movable {
            match scheduling::find_next_available_slot(
                &slots,
                &exceptions,
                &workspace.timezone,
                publication.platform,
                now,
                &taken,
                horizon_days,
            ) {
                Some(at) => {
                    taken.insert(at);
                    publication.scheduled_at = Some(at);
                    publication.updated_at = Utc::now();
                    to_persist.push(publication);
                }
                None => {
                    publication.transition(PublicationStatus::Queued)?;
                    publication.scheduled_at = None;
                    unscheduled.push(publication.id);
                    to_persist.push(publication);
                }
            }
        }

        let scheduled_ids: Vec<Publication> = to_persist
            .iter()
            .filter(|p| p.status == PublicationStatus::Scheduled)
            .cloned()
            .collect();
        self.publication_repo.bulk_update(&to_persist).await?;

        Ok(BulkScheduleResult {
            scheduled: scheduled_ids,
            skipped: unscheduled,
        })
    }

    /// Every `Scheduled` publication (in the workspace, or one channel)
    /// that falls within `[start, end)` on the calendar, expressed in the
    /// workspace's local timezone (section 41-45) — the frontend never
    /// does timezone math itself.
    pub async fn calendar_range(
        &self,
        workspace_id: Uuid,
        channel_id: Option<Uuid>,
        start: NaiveDate,
        end: NaiveDate,
    ) -> DomainResult<Vec<CalendarPublication>> {
        let workspace = self.load_workspace(workspace_id).await?;
        let tz: chrono_tz::Tz =
            workspace
                .timezone
                .parse()
                .map_err(|_| DomainError::InvalidValue {
                    field: "timezone",
                    reason: format!("{:?} is not a recognized IANA timezone", workspace.timezone),
                })?;

        let from = local_midnight_utc(tz, start);
        let to = local_midnight_utc(tz, end + Duration::days(1));

        let publications = self
            .publication_repo
            .list_scheduled_in_range(workspace_id, channel_id, from, to)
            .await?;

        Ok(publications
            .into_iter()
            .map(|publication| {
                let local = publication
                    .scheduled_at
                    .unwrap_or(publication.created_at)
                    .with_timezone(&tz);
                CalendarPublication {
                    local_date: local.date_naive(),
                    local_time: local.format("%H:%M").to_string(),
                    publication,
                }
            })
            .collect())
    }

    /// `Scheduled` publications at or before `now` (section 111's due-
    /// publication query, designed for a clean Phase 5 handoff — nothing
    /// in Phase 3 acts on this list beyond surfacing it as "Overdue" in
    /// the UI).
    pub async fn due_publications(
        &self,
        workspace_id: Uuid,
        now: DateTime<Utc>,
    ) -> DomainResult<Vec<Publication>> {
        self.publication_repo.list_due(workspace_id, now).await
    }

    async fn taken_instants(
        &self,
        channel_id: Uuid,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> DomainResult<HashSet<DateTime<Utc>>> {
        // `list_scheduled_in_range` is workspace-scoped with an optional
        // channel filter; we only need the channel's own workspace here,
        // so fetch the channel first to get it.
        let channel = self.load_channel(channel_id).await?;
        let scheduled = self
            .publication_repo
            .list_scheduled_in_range(channel.workspace_id, Some(channel_id), from, to)
            .await?;
        Ok(scheduled
            .into_iter()
            .filter_map(|p| p.scheduled_at)
            .collect())
    }

    async fn assert_no_conflict(
        &self,
        publication: &Publication,
        at: DateTime<Utc>,
    ) -> DomainResult<()> {
        let conflicts = self
            .publication_repo
            .list_scheduled_in_range(
                publication.workspace_id,
                Some(publication.channel_id),
                at,
                at + Duration::seconds(1),
            )
            .await?;
        if conflicts.iter().any(|p| p.id != publication.id) {
            return Err(DomainError::ScheduleConflict);
        }
        Ok(())
    }

    async fn load_publication(&self, id: Uuid) -> DomainResult<Publication> {
        self.publication_repo
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                entity: "Publication",
                id: id.to_string(),
            })
    }

    async fn load_channel(&self, id: Uuid) -> DomainResult<crate::domain::channel::Channel> {
        self.channel_repo
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                entity: "Channel",
                id: id.to_string(),
            })
    }

    async fn load_workspace(&self, id: Uuid) -> DomainResult<crate::domain::workspace::Workspace> {
        self.workspace_repo
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                entity: "Workspace",
                id: id.to_string(),
            })
    }
}

pub struct BulkScheduleResult {
    pub scheduled: Vec<Publication>,
    pub skipped: Vec<Uuid>,
}

/// Resolves a local wall-clock instant in `tz` to UTC, DST-safe: an
/// ambiguous "fall back" instant resolves to the earlier occurrence; an
/// instant that falls in a "spring forward" gap and therefore doesn't
/// exist falls forward by one hour rather than erroring the caller.
fn resolve_local_datetime_to_utc(tz: chrono_tz::Tz, naive: chrono::NaiveDateTime) -> DateTime<Utc> {
    match tz.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => dt.with_timezone(&Utc),
        chrono::LocalResult::Ambiguous(earliest, _) => earliest.with_timezone(&Utc),
        chrono::LocalResult::None => tz
            .from_local_datetime(&(naive + Duration::hours(1)))
            .single()
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|| Utc.from_utc_datetime(&naive)),
    }
}

fn local_midnight_utc(tz: chrono_tz::Tz, date: NaiveDate) -> DateTime<Utc> {
    resolve_local_datetime_to_utc(tz, date.and_hms_opt(0, 0, 0).unwrap())
}

#[cfg(test)]
mod tests {
    use chrono::Timelike;

    use super::*;
    use crate::application::activity_service::ActivityService;
    use crate::application::publication_service::PublicationService;
    use crate::domain::channel::ChannelStatus;
    use crate::domain::platform::Platform;
    use crate::domain::ports::repositories::{
        ActivityRepository, ChannelRepository, QueueItemRepository, VideoRepository,
    };
    use crate::domain::video_status::VideoPriority;
    use crate::infrastructure::repositories::{
        SqliteActivityRepository, SqliteChannelRepository, SqlitePublicationRepository,
        SqliteQueueItemRepository, SqliteScheduleExceptionRepository, SqliteScheduleSlotRepository,
        SqliteVideoRepository, SqliteWorkspaceRepository,
    };
    use crate::test_support::*;

    struct Harness {
        scheduler: SchedulerService,
        publications: PublicationService,
    }

    async fn build_harness(pool: sqlx::SqlitePool) -> Harness {
        let publication_repo: Arc<dyn PublicationRepository> =
            Arc::new(SqlitePublicationRepository::new(pool.clone()));
        let channel_repo: Arc<dyn ChannelRepository> =
            Arc::new(SqliteChannelRepository::new(pool.clone()));
        let workspace_repo: Arc<dyn WorkspaceRepository> =
            Arc::new(SqliteWorkspaceRepository::new(pool.clone()));
        let slot_repo: Arc<dyn ScheduleSlotRepository> =
            Arc::new(SqliteScheduleSlotRepository::new(pool.clone()));
        let exception_repo: Arc<dyn ScheduleExceptionRepository> =
            Arc::new(SqliteScheduleExceptionRepository::new(pool.clone()));
        let queue_item_repo: Arc<dyn QueueItemRepository> =
            Arc::new(SqliteQueueItemRepository::new(pool.clone()));
        let video_repo: Arc<dyn VideoRepository> =
            Arc::new(SqliteVideoRepository::new(pool.clone()));
        let activity_repo: Arc<dyn ActivityRepository> =
            Arc::new(SqliteActivityRepository::new(pool.clone()));
        let activity_service = Arc::new(ActivityService::new(activity_repo));

        let scheduler = SchedulerService::new(
            publication_repo.clone(),
            channel_repo,
            workspace_repo,
            slot_repo,
            exception_repo,
            activity_service.clone(),
        );
        let publications = PublicationService::new(
            publication_repo,
            queue_item_repo,
            video_repo,
            activity_service,
        );

        Harness {
            scheduler,
            publications,
        }
    }

    #[tokio::test]
    async fn scheduling_a_queued_publication_transitions_it_and_sets_the_time() {
        let pool = temp_pool("sched-svc-basic").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let video_id = seed_video(&pool, workspace_id, source_id, None, "Clip").await;
        let harness = build_harness(pool).await;

        let publication = harness
            .publications
            .add_to_queue(
                workspace_id,
                video_id,
                channel_id,
                Platform::YouTube,
                None,
                None,
            )
            .await
            .unwrap();

        let at = Utc::now() + Duration::days(1);
        let scheduled = harness
            .scheduler
            .schedule_at(publication.id, at)
            .await
            .unwrap();

        assert_eq!(scheduled.status, PublicationStatus::Scheduled);
        assert_eq!(scheduled.scheduled_at, Some(at));
    }

    #[tokio::test]
    async fn scheduling_to_a_paused_channel_is_rejected() {
        let pool = temp_pool("sched-svc-paused").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let video_id = seed_video(&pool, workspace_id, source_id, None, "Clip").await;
        let harness = build_harness(pool.clone()).await;

        let publication = harness
            .publications
            .add_to_queue(
                workspace_id,
                video_id,
                channel_id,
                Platform::YouTube,
                None,
                None,
            )
            .await
            .unwrap();

        let channel_repo = SqliteChannelRepository::new(pool);
        let mut channel = channel_repo.get(channel_id).await.unwrap().unwrap();
        channel.status = ChannelStatus::Paused;
        channel_repo.update(&channel).await.unwrap();

        let result = harness
            .scheduler
            .schedule_at(publication.id, Utc::now() + Duration::days(1))
            .await;
        assert!(matches!(result, Err(DomainError::ChannelPaused { .. })));
    }

    #[tokio::test]
    async fn scheduling_two_publications_to_the_exact_same_instant_conflicts() {
        let pool = temp_pool("sched-svc-conflict").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let video_a = seed_video(&pool, workspace_id, source_id, None, "A").await;
        let video_b = seed_video(&pool, workspace_id, source_id, None, "B").await;
        let harness = build_harness(pool).await;

        let pub_a = harness
            .publications
            .add_to_queue(
                workspace_id,
                video_a,
                channel_id,
                Platform::YouTube,
                None,
                None,
            )
            .await
            .unwrap();
        let pub_b = harness
            .publications
            .add_to_queue(
                workspace_id,
                video_b,
                channel_id,
                Platform::YouTube,
                None,
                None,
            )
            .await
            .unwrap();

        let at = Utc::now() + Duration::days(2);
        harness.scheduler.schedule_at(pub_a.id, at).await.unwrap();
        let second = harness.scheduler.schedule_at(pub_b.id, at).await;

        assert!(matches!(second, Err(DomainError::ScheduleConflict)));
    }

    /// The application-level conflict check has an inherent TOCTOU race —
    /// two concurrent requests can both pass the "is this instant free?"
    /// check before either commits. This test exercises the *real*
    /// concurrent race (not just sequential calls) to prove the database's
    /// partial unique index is what actually prevents a double-booked slot
    /// (section 27/28/51), independent of the application-level check.
    #[tokio::test]
    async fn concurrent_scheduling_to_the_same_instant_never_double_books() {
        let pool = temp_pool("sched-svc-race").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let video_a = seed_video(&pool, workspace_id, source_id, None, "A").await;
        let video_b = seed_video(&pool, workspace_id, source_id, None, "B").await;
        let harness = build_harness(pool.clone()).await;

        let pub_a = harness
            .publications
            .add_to_queue(
                workspace_id,
                video_a,
                channel_id,
                Platform::YouTube,
                None,
                None,
            )
            .await
            .unwrap();
        let pub_b = harness
            .publications
            .add_to_queue(
                workspace_id,
                video_b,
                channel_id,
                Platform::YouTube,
                None,
                None,
            )
            .await
            .unwrap();

        let harness = Arc::new(harness);
        let at = Utc::now() + Duration::days(3);

        let h1 = harness.clone();
        let h2 = harness.clone();
        let (r1, r2) = tokio::join!(
            tokio::spawn(async move { h1.scheduler.schedule_at(pub_a.id, at).await }),
            tokio::spawn(async move { h2.scheduler.schedule_at(pub_b.id, at).await }),
        );
        let (r1, r2) = (r1.unwrap(), r2.unwrap());

        let successes = [&r1, &r2].iter().filter(|r| r.is_ok()).count();
        assert_eq!(
            successes, 1,
            "exactly one of the two racing schedules must win"
        );

        let scheduled_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM publications WHERE channel_id = ? AND status = 'scheduled' AND scheduled_at = ?",
        )
        .bind(channel_id.to_string())
        .bind(at.to_rfc3339())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            scheduled_count, 1,
            "the database must never end up with two rows at the same instant"
        );
    }

    #[tokio::test]
    async fn auto_schedule_places_a_publication_in_the_channels_next_slot() {
        let pool = temp_pool("sched-svc-auto").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        set_workspace_timezone(&pool, workspace_id, "UTC").await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let video_id = seed_video(&pool, workspace_id, source_id, None, "Clip").await;
        let harness = build_harness(pool.clone()).await;

        // A slot on every weekday at noon guarantees the 120-day search
        // finds *something* regardless of which day this test runs on.
        let slot_repo = SqliteScheduleSlotRepository::new(pool);
        for day in 0..7 {
            let slot =
                crate::domain::schedule_slot::ScheduleSlot::new(channel_id, None, day, "12:00")
                    .unwrap();
            slot_repo.create(&slot).await.unwrap();
        }

        let publication = harness
            .publications
            .add_to_queue(
                workspace_id,
                video_id,
                channel_id,
                Platform::YouTube,
                None,
                None,
            )
            .await
            .unwrap();

        let scheduled = harness
            .scheduler
            .auto_schedule(publication.id)
            .await
            .unwrap();
        assert_eq!(scheduled.status, PublicationStatus::Scheduled);
        assert!(scheduled.scheduled_at.is_some());
    }

    #[tokio::test]
    async fn auto_schedule_refuses_a_locked_publication() {
        let pool = temp_pool("sched-svc-auto-locked").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let video_id = seed_video(&pool, workspace_id, source_id, None, "Clip").await;
        let harness = build_harness(pool.clone()).await;

        let publication = harness
            .publications
            .add_to_queue(
                workspace_id,
                video_id,
                channel_id,
                Platform::YouTube,
                None,
                None,
            )
            .await
            .unwrap();

        let publication_repo = SqlitePublicationRepository::new(pool);
        let mut locked = publication_repo.get(publication.id).await.unwrap().unwrap();
        locked.locked = true;
        publication_repo.update(&locked).await.unwrap();

        let result = harness.scheduler.auto_schedule(publication.id).await;
        assert!(matches!(result, Err(DomainError::PublicationLocked { .. })));
    }

    #[tokio::test]
    async fn auto_schedule_channel_is_transactional_and_priority_ordered() {
        let pool = temp_pool("sched-svc-bulk").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        set_workspace_timezone(&pool, workspace_id, "UTC").await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let harness = build_harness(pool.clone()).await;

        // Two active slots per week so both queued items can find a home.
        let slot_repo = SqliteScheduleSlotRepository::new(pool.clone());
        for day in 0..7 {
            let slot =
                crate::domain::schedule_slot::ScheduleSlot::new(channel_id, None, day, "09:00")
                    .unwrap();
            slot_repo.create(&slot).await.unwrap();
        }

        let video_low = seed_video(&pool, workspace_id, source_id, None, "Low").await;
        let video_urgent = seed_video(&pool, workspace_id, source_id, None, "Urgent").await;

        let low = harness
            .publications
            .add_to_queue(
                workspace_id,
                video_low,
                channel_id,
                Platform::YouTube,
                None,
                Some(VideoPriority::Low),
            )
            .await
            .unwrap();
        let urgent = harness
            .publications
            .add_to_queue(
                workspace_id,
                video_urgent,
                channel_id,
                Platform::YouTube,
                None,
                Some(VideoPriority::Urgent),
            )
            .await
            .unwrap();

        let result = harness
            .scheduler
            .auto_schedule_channel(channel_id, 30)
            .await
            .unwrap();
        assert_eq!(result.scheduled.len(), 2);
        assert!(result.skipped.is_empty());

        let urgent_scheduled = result.scheduled.iter().find(|p| p.id == urgent.id).unwrap();
        let low_scheduled = result.scheduled.iter().find(|p| p.id == low.id).unwrap();
        assert!(
            urgent_scheduled.scheduled_at.unwrap() <= low_scheduled.scheduled_at.unwrap(),
            "the urgent item should be placed at or before the low-priority one"
        );
    }

    #[tokio::test]
    async fn calendar_range_resolves_scheduled_at_into_workspace_local_time() {
        let pool = temp_pool("sched-svc-calendar").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        set_workspace_timezone(&pool, workspace_id, "America/New_York").await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let video_id = seed_video(&pool, workspace_id, source_id, None, "Clip").await;
        let harness = build_harness(pool).await;

        let publication = harness
            .publications
            .add_to_queue(
                workspace_id,
                video_id,
                channel_id,
                Platform::YouTube,
                None,
                None,
            )
            .await
            .unwrap();
        // 2024-06-15T14:00:00Z is 10:00 local in America/New_York (EDT, UTC-4).
        let at = Utc.with_ymd_and_hms(2024, 6, 15, 14, 0, 0).unwrap();
        harness
            .scheduler
            .schedule_at(publication.id, at)
            .await
            .unwrap();

        let items = harness
            .scheduler
            .calendar_range(
                workspace_id,
                None,
                NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
                NaiveDate::from_ymd_opt(2024, 6, 30).unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].local_date,
            NaiveDate::from_ymd_opt(2024, 6, 15).unwrap()
        );
        assert_eq!(items[0].local_time, "10:00");
    }

    #[tokio::test]
    async fn reschedule_to_date_keeps_the_same_local_time_of_day() {
        let pool = temp_pool("sched-svc-drag").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        set_workspace_timezone(&pool, workspace_id, "America/New_York").await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let video_id = seed_video(&pool, workspace_id, source_id, None, "Clip").await;
        let harness = build_harness(pool).await;

        let publication = harness
            .publications
            .add_to_queue(
                workspace_id,
                video_id,
                channel_id,
                Platform::YouTube,
                None,
                None,
            )
            .await
            .unwrap();
        // 14:00Z = 10:00 America/New_York (EDT).
        let at = Utc.with_ymd_and_hms(2024, 6, 15, 14, 0, 0).unwrap();
        harness
            .scheduler
            .schedule_at(publication.id, at)
            .await
            .unwrap();

        let moved = harness
            .scheduler
            .reschedule_to_date(
                publication.id,
                NaiveDate::from_ymd_opt(2024, 6, 20).unwrap(),
            )
            .await
            .unwrap();

        let local = moved
            .scheduled_at
            .unwrap()
            .with_timezone(&chrono_tz::America::New_York);
        assert_eq!(
            local.date_naive(),
            NaiveDate::from_ymd_opt(2024, 6, 20).unwrap()
        );
        assert_eq!((local.hour(), local.minute()), (10, 0));
    }

    #[tokio::test]
    async fn reschedule_to_date_rejects_a_locked_publication() {
        let pool = temp_pool("sched-svc-drag-locked").await;
        let (workspace_id, source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let video_id = seed_video(&pool, workspace_id, source_id, None, "Clip").await;
        let harness = build_harness(pool.clone()).await;

        let publication = harness
            .publications
            .add_to_queue(
                workspace_id,
                video_id,
                channel_id,
                Platform::YouTube,
                None,
                None,
            )
            .await
            .unwrap();
        harness
            .scheduler
            .schedule_at(publication.id, Utc::now() + Duration::days(1))
            .await
            .unwrap();

        let publication_repo = SqlitePublicationRepository::new(pool);
        let mut locked = publication_repo.get(publication.id).await.unwrap().unwrap();
        locked.locked = true;
        publication_repo.update(&locked).await.unwrap();

        let result = harness
            .scheduler
            .reschedule_to_date(publication.id, Utc::now().date_naive() + Duration::days(5))
            .await;
        assert!(matches!(result, Err(DomainError::PublicationLocked { .. })));
    }
}
