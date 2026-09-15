use std::sync::Arc;

use chrono::NaiveDate;
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform::Platform;
use crate::domain::ports::repositories::{ScheduleExceptionRepository, ScheduleSlotRepository};
use crate::domain::schedule_exception::ScheduleException;
use crate::domain::schedule_slot::ScheduleSlot;

/// Manages a channel's recurring weekly schedule (section 16) and its
/// date-specific exceptions (section 17). Pure CRUD orchestration — the
/// actual "where does the next publication land" logic lives in
/// `domain::scheduling` / `SchedulerService`.
pub struct ScheduleSlotService {
    slot_repo: Arc<dyn ScheduleSlotRepository>,
    exception_repo: Arc<dyn ScheduleExceptionRepository>,
}

impl ScheduleSlotService {
    pub fn new(
        slot_repo: Arc<dyn ScheduleSlotRepository>,
        exception_repo: Arc<dyn ScheduleExceptionRepository>,
    ) -> Self {
        Self {
            slot_repo,
            exception_repo,
        }
    }

    pub async fn list_for_channel(&self, channel_id: Uuid) -> DomainResult<Vec<ScheduleSlot>> {
        self.slot_repo.list_for_channel(channel_id).await
    }

    pub async fn create_slot(
        &self,
        channel_id: Uuid,
        platform: Option<Platform>,
        day_of_week: i32,
        time_of_day: String,
    ) -> DomainResult<ScheduleSlot> {
        let slot = ScheduleSlot::new(channel_id, platform, day_of_week, time_of_day)?;
        self.slot_repo.create(&slot).await?;
        Ok(slot)
    }

    pub async fn set_slot_active(&self, id: Uuid, is_active: bool) -> DomainResult<ScheduleSlot> {
        let mut slot = self.load_slot(id).await?;
        slot.is_active = is_active;
        slot.updated_at = chrono::Utc::now();
        self.slot_repo.update(&slot).await?;
        Ok(slot)
    }

    pub async fn delete_slot(&self, id: Uuid) -> DomainResult<()> {
        self.slot_repo.delete(id).await
    }

    /// Copies every active slot from `from_day` onto each day in `to_days`
    /// (section 16's "copy day" / "copy to weekdays" shortcuts). Slots that
    /// would collide with an existing one on the target day are silently
    /// skipped (the database's own uniqueness guarantee) rather than
    /// failing the whole copy.
    pub async fn copy_day(
        &self,
        channel_id: Uuid,
        from_day: i32,
        to_days: Vec<i32>,
    ) -> DomainResult<Vec<ScheduleSlot>> {
        let source_slots: Vec<ScheduleSlot> = self
            .slot_repo
            .list_for_channel(channel_id)
            .await?
            .into_iter()
            .filter(|s| s.is_active && s.day_of_week == from_day)
            .collect();

        let mut created = Vec::new();
        for day in to_days {
            if day == from_day {
                continue;
            }
            for source in &source_slots {
                let slot = ScheduleSlot::new(
                    channel_id,
                    source.platform,
                    day,
                    source.time_of_day.clone(),
                )?;
                if self.slot_repo.create(&slot).await.is_ok() {
                    created.push(slot);
                }
            }
        }
        Ok(created)
    }

    pub async fn add_skip_exception(
        &self,
        channel_id: Uuid,
        date: NaiveDate,
        reason: Option<String>,
    ) -> DomainResult<ScheduleException> {
        let exception = ScheduleException::new_skip(channel_id, date, reason);
        self.exception_repo.create(&exception).await?;
        Ok(exception)
    }

    pub async fn remove_exception(&self, id: Uuid) -> DomainResult<()> {
        self.exception_repo.delete(id).await
    }

    pub async fn list_exceptions_in_range(
        &self,
        channel_id: Uuid,
        from: NaiveDate,
        to: NaiveDate,
    ) -> DomainResult<Vec<ScheduleException>> {
        self.exception_repo
            .list_for_channel_in_range(channel_id, from, to)
            .await
    }

    async fn load_slot(&self, id: Uuid) -> DomainResult<ScheduleSlot> {
        self.slot_repo
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                entity: "ScheduleSlot",
                id: id.to_string(),
            })
    }
}
