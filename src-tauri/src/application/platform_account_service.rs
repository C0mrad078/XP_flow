use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::platform::Platform;
use crate::domain::platform_account::{PlatformAccount, PlatformAccountStatus};
use crate::domain::ports::repositories::PlatformAccountRepository;

/// Plain CRUD/status orchestration for `PlatformAccount` rows. The actual
/// OAuth business logic (talking to a provider/broker, exchanging codes,
/// mapping identity) lives in `PlatformAuthService` — this service only
/// ever mutates the persisted record, never performs network I/O, which is
/// what keeps it trivially testable and reusable by both the auth flow and
/// plain account-management actions (rename target, disconnect, etc).
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

    pub async fn list_for_workspace(
        &self,
        workspace_id: Uuid,
    ) -> DomainResult<Vec<PlatformAccount>> {
        self.repo.list_for_workspace(workspace_id).await
    }

    pub async fn get(&self, id: Uuid) -> DomainResult<PlatformAccount> {
        self.repo
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound {
                entity: "PlatformAccount",
                id: id.to_string(),
            })
    }

    /// Creates an empty, `NotConfigured` account row — the placeholder a
    /// "Connect" button in the UI targets before any OAuth flow starts.
    pub async fn create(
        &self,
        workspace_id: Uuid,
        channel_id: Uuid,
        platform: Platform,
    ) -> DomainResult<PlatformAccount> {
        let account = PlatformAccount::new(workspace_id, channel_id, platform);
        self.repo.create(&account).await?;
        Ok(account)
    }

    /// Marks `id` as the default target for its channel/platform pair,
    /// unsetting any previous default among its siblings (section 49).
    pub async fn set_default(&self, id: Uuid) -> DomainResult<PlatformAccount> {
        let mut account = self.get(id).await?;

        for sibling in self.repo.list_for_channel(account.channel_id).await? {
            if sibling.id != account.id
                && sibling.platform == account.platform
                && sibling.default_target
            {
                let mut sibling = sibling;
                sibling.default_target = false;
                sibling.updated_at = Utc::now();
                self.repo.update(&sibling).await?;
            }
        }

        account.default_target = true;
        account.updated_at = Utc::now();
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

    /// Reassigns a connected account to a different channel (section 54).
    /// Purely a foreign-key move — the provider connection itself is
    /// untouched.
    pub async fn reassign_channel(
        &self,
        id: Uuid,
        new_channel_id: Uuid,
    ) -> DomainResult<PlatformAccount> {
        let mut account = self.get(id).await?;
        account.channel_id = new_channel_id;
        account.default_target = false;
        account.updated_at = Utc::now();
        self.repo.update(&account).await?;
        Ok(account)
    }

    /// Marks an account disconnected without deleting its row (section
    /// 56/57 — history and any scheduled publications referencing it must
    /// survive). Provider-side revocation and broker/keychain credential
    /// cleanup are the caller's (`PlatformAuthService`) responsibility
    /// before this is called.
    pub async fn mark_disconnected(&self, id: Uuid) -> DomainResult<PlatformAccount> {
        let mut account = self.get(id).await?;
        account.status = PlatformAccountStatus::Revoked;
        account.provider_connection_id = None;
        account.access_expires_at = None;
        account.refresh_expires_at = None;
        account.updated_at = Utc::now();
        self.repo.update(&account).await?;
        Ok(account)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::repositories::SqlitePlatformAccountRepository;
    use crate::test_support::*;

    #[tokio::test]
    async fn reassign_channel_persists_the_new_channel_id() {
        let pool = temp_pool("platform-account-svc-reassign").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let channel_a = seed_channel(&pool, workspace_id, "Channel A").await;
        let channel_b = seed_channel(&pool, workspace_id, "Channel B").await;
        let repo = Arc::new(SqlitePlatformAccountRepository::new(pool));
        let service =
            PlatformAccountService::new(repo.clone() as Arc<dyn PlatformAccountRepository>);

        let account = service
            .create(workspace_id, channel_a, Platform::YouTube)
            .await
            .unwrap();

        let reassigned = service
            .reassign_channel(account.id, channel_b)
            .await
            .unwrap();
        assert_eq!(reassigned.channel_id, channel_b);

        // Round-trips through a fresh read, not just the in-memory struct
        // `reassign_channel` returns — this is what caught the original
        // bug, where the repository's UPDATE silently omitted channel_id.
        let reloaded = service.get(account.id).await.unwrap();
        assert_eq!(reloaded.channel_id, channel_b);
        assert!(repo.list_for_channel(channel_a).await.unwrap().is_empty());
        assert_eq!(repo.list_for_channel(channel_b).await.unwrap().len(), 1);
    }
}
