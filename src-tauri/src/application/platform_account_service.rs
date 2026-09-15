use std::sync::Arc;

use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform::Platform;
use crate::domain::platform_account::PlatformAccount;
use crate::domain::ports::repositories::PlatformAccountRepository;

/// Manages the (still-unconnected, no real OAuth) `PlatformAccount` rows a
/// channel uses as "Add to Queue" targets (section 49). Connecting a real
/// account is out of scope for Phase 3 (section 111) — this only lets an
/// operator register the intended target and mark a default.
pub struct PlatformAccountService {
    repo: Arc<dyn PlatformAccountRepository>,
}

impl PlatformAccountService {
    pub fn new(repo: Arc<dyn PlatformAccountRepository>) -> Self {
        Self { repo }
    }

    pub async fn list_for_channel(&self, channel_id: Uuid) -> DomainResult<Vec<PlatformAccount>> {
        self.repo.list_for_channel(channel_id).await
    }

    pub async fn create(
        &self,
        channel_id: Uuid,
        platform: Platform,
    ) -> DomainResult<PlatformAccount> {
        let account = PlatformAccount::new(channel_id, platform);
        self.repo.create(&account).await?;
        Ok(account)
    }

    /// Marks `id` as the default target for its channel/platform pair,
    /// unsetting any previous default among its siblings (section 49).
    pub async fn set_default(&self, id: Uuid) -> DomainResult<PlatformAccount> {
        let mut account = self
            .repo
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                entity: "PlatformAccount",
                id: id.to_string(),
            })?;

        for sibling in self.repo.list_for_channel(account.channel_id).await? {
            if sibling.id != account.id
                && sibling.platform == account.platform
                && sibling.default_target
            {
                let mut sibling = sibling;
                sibling.default_target = false;
                sibling.updated_at = chrono::Utc::now();
                self.repo.update(&sibling).await?;
            }
        }

        account.default_target = true;
        account.updated_at = chrono::Utc::now();
        self.repo.update(&account).await?;
        Ok(account)
    }

    pub async fn get_default_for_channel_platform(
        &self,
        channel_id: Uuid,
        platform: Platform,
    ) -> DomainResult<Option<PlatformAccount>> {
        self.repo
            .get_default_for_channel_platform(channel_id, platform)
            .await
    }
}
