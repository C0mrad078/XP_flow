use std::collections::HashMap;
use std::sync::Arc;

use uuid::Uuid;

use crate::domain::channel::{Channel, ChannelStatus};
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform_account::PlatformAccount;
use crate::domain::ports::repositories::{
    ChannelRepository, PlatformAccountRepository, PublicationRepository, ScheduleSlotRepository,
};

/// Section 96's aggregate — everything a Channels-screen card needs, in
/// one query set per field regardless of how many channels the workspace
/// has (`ChannelService::list_operational_overview`), replacing the
/// Phase 3 pattern of each `ChannelCard` firing 3 of its own IPC round
/// trips.
#[derive(serde::Serialize)]
pub struct ChannelOverview {
    pub channel: Channel,
    pub queued_count: i64,
    pub active_slot_count: i64,
    pub platform_accounts: Vec<PlatformAccount>,
}

/// Channel management, including the operational-overview aggregate the
/// Channels screen renders from (section 96).
pub struct ChannelService {
    channel_repo: Arc<dyn ChannelRepository>,
    publication_repo: Arc<dyn PublicationRepository>,
    schedule_slot_repo: Arc<dyn ScheduleSlotRepository>,
    platform_account_repo: Arc<dyn PlatformAccountRepository>,
}

impl ChannelService {
    pub fn new(
        channel_repo: Arc<dyn ChannelRepository>,
        publication_repo: Arc<dyn PublicationRepository>,
        schedule_slot_repo: Arc<dyn ScheduleSlotRepository>,
        platform_account_repo: Arc<dyn PlatformAccountRepository>,
    ) -> Self {
        Self {
            channel_repo,
            publication_repo,
            schedule_slot_repo,
            platform_account_repo,
        }
    }

    pub async fn list(&self, workspace_id: Uuid) -> DomainResult<Vec<Channel>> {
        self.channel_repo.list_for_workspace(workspace_id).await
    }

    pub async fn create(&self, workspace_id: Uuid, name: String) -> DomainResult<Channel> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(DomainError::Validation(
                "channel name cannot be empty".into(),
            ));
        }
        let channel = Channel::new(workspace_id, trimmed);
        self.channel_repo.create(&channel).await?;
        Ok(channel)
    }

    pub async fn get(&self, id: Uuid) -> DomainResult<Option<Channel>> {
        self.channel_repo.get(id).await
    }

    /// Pauses or resumes a channel (section 23). A paused channel keeps
    /// all of its data untouched — it is simply skipped by the scheduler
    /// and excluded from "next available slot" search until resumed.
    pub async fn set_status(&self, id: Uuid, status: ChannelStatus) -> DomainResult<Channel> {
        let mut channel =
            self.channel_repo
                .get(id)
                .await?
                .ok_or_else(|| DomainError::NotFound {
                    entity: "Channel",
                    id: id.to_string(),
                })?;
        channel.status = status;
        channel.updated_at = chrono::Utc::now();
        self.channel_repo.update(&channel).await?;
        Ok(channel)
    }

    /// Section 96: every channel in the workspace plus its queued-video
    /// count, active-slot count and platform accounts, computed from
    /// exactly four workspace-scoped queries total — never one query set
    /// per channel. `ChannelCard` in the frontend calls this once instead
    /// of firing 3 of its own IPC round trips per card.
    pub async fn list_operational_overview(
        &self,
        workspace_id: Uuid,
    ) -> DomainResult<Vec<ChannelOverview>> {
        let channels = self.channel_repo.list_for_workspace(workspace_id).await?;
        let queued_counts: HashMap<Uuid, i64> = self
            .publication_repo
            .count_active_grouped_by_channel(workspace_id)
            .await?
            .into_iter()
            .collect();
        let slot_counts: HashMap<Uuid, i64> = self
            .schedule_slot_repo
            .count_active_grouped_by_channel(workspace_id)
            .await?
            .into_iter()
            .collect();
        let mut accounts_by_channel: HashMap<Uuid, Vec<PlatformAccount>> = HashMap::new();
        for account in self
            .platform_account_repo
            .list_for_workspace(workspace_id)
            .await?
        {
            accounts_by_channel
                .entry(account.channel_id)
                .or_default()
                .push(account);
        }

        Ok(channels
            .into_iter()
            .map(|channel| {
                let id = channel.id;
                ChannelOverview {
                    queued_count: queued_counts.get(&id).copied().unwrap_or(0),
                    active_slot_count: slot_counts.get(&id).copied().unwrap_or(0),
                    platform_accounts: accounts_by_channel.remove(&id).unwrap_or_default(),
                    channel,
                }
            })
            .collect())
    }
}
