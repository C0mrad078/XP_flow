use std::sync::Arc;

use uuid::Uuid;

use crate::domain::channel::{Channel, ChannelStatus};
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::ports::repositories::ChannelRepository;

/// Minimal channel management — Phase 1 built the `Channel`
/// domain/repository but exposed no commands (the Channels *screen*
/// stayed mock, deliberately, per section 29 of the Phase 1 brief).
/// Phase 2's channel-assignment features (sections 9, 46) are the first
/// thing that actually needs real channel rows to assign a
/// [`crate::domain::video::Video`] or [`crate::domain::video_source::VideoSource`]
/// to — this service is deliberately small: list + create, nothing the
/// full Channels screen will eventually need (platform accounts, health,
/// etc.) is here.
pub struct ChannelService {
    channel_repo: Arc<dyn ChannelRepository>,
}

impl ChannelService {
    pub fn new(channel_repo: Arc<dyn ChannelRepository>) -> Self {
        Self { channel_repo }
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
}
