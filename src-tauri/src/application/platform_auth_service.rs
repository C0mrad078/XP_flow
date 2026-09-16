use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;

use crate::application::activity_service::ActivityService;
use crate::domain::activity_event::{ActivityCategory, ActivityLevel};
use crate::domain::auth_error::AuthError;
use crate::domain::capability::map_scopes_to_capabilities;
use crate::domain::notification::NotificationType;
use crate::domain::oauth::pkce::generate_pkce;
use crate::domain::oauth::{AuthFlowState, AuthSession};
use crate::domain::platform::Platform;
use crate::domain::platform_account::{PlatformAccount, PlatformAccountStatus};
use crate::domain::ports::platform_auth_provider::PlatformAuthProvider;
use crate::domain::ports::platform_connector::PlatformConnector;
use crate::domain::ports::repositories::PlatformAccountRepository;
use crate::domain::provider_identity::ConnectedIdentity;
use crate::services::notification_service::NotificationService;

struct Flight {
    cancel: Option<oneshot::Sender<()>>,
    state: AuthFlowState,
}

/// How long a finished flight's terminal state stays pollable before its
/// map entry is dropped (see the eviction comment in `start`).
const FLIGHT_RETENTION: std::time::Duration = std::time::Duration::from_secs(60);

/// Orchestrates the OAuth connect/reconnect/disconnect/validate/refresh
/// lifecycle across all three providers (section 4's shared-abstraction-
/// but-provider-specific-behavior split lives one layer down, in
/// `PlatformAuthProvider`/`PlatformConnector` — this service is what turns
/// "the user clicked Connect" into a persisted `PlatformAccount`).
///
/// A connect attempt runs as a detached background task (`begin_connect`
/// returns immediately with a session id) rather than blocking the
/// calling Tauri command for as long as the user takes to authorize in
/// their browser — section 44 wants a live, pollable UI state
/// (`AuthFlowState`) and section 45 wants the user to be able to cancel,
/// neither of which fits a single blocking request/response call.
pub struct PlatformAuthService {
    auth_providers: HashMap<Platform, Arc<dyn PlatformAuthProvider>>,
    connectors: HashMap<Platform, Arc<dyn PlatformConnector>>,
    platform_account_repo: Arc<dyn PlatformAccountRepository>,
    activity_service: Arc<ActivityService>,
    notification_service: Arc<NotificationService>,
    flights: Arc<Mutex<HashMap<Uuid, Flight>>>,
}

impl PlatformAuthService {
    pub fn new(
        auth_providers: HashMap<Platform, Arc<dyn PlatformAuthProvider>>,
        connectors: HashMap<Platform, Arc<dyn PlatformConnector>>,
        platform_account_repo: Arc<dyn PlatformAccountRepository>,
        activity_service: Arc<ActivityService>,
        notification_service: Arc<NotificationService>,
    ) -> Self {
        Self {
            auth_providers,
            connectors,
            platform_account_repo,
            activity_service,
            notification_service,
            flights: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Starts a brand-new connection attempt for a channel/platform that
    /// has no account yet. Returns the session id to poll/cancel.
    pub async fn begin_connect(
        &self,
        workspace_id: Uuid,
        channel_id: Uuid,
        platform: Platform,
    ) -> Result<Uuid, AuthError> {
        self.start(workspace_id, channel_id, platform, None, false)
            .await
    }

    /// Re-runs the authorization flow for an existing account (section
    /// 58) — a `ReauthRequired`/`Revoked` account, or simply a user-
    /// requested "Reconnect". `allow_identity_change` must be explicitly
    /// true (a second call after the user confirms a prompt) to accept a
    /// different real account than the one already on this row (section
    /// 59) — the first attempt always fails closed with
    /// `AccountIdentityMismatch` instead of silently overwriting.
    pub async fn begin_reconnect(
        &self,
        account_id: Uuid,
        allow_identity_change: bool,
    ) -> Result<Uuid, AuthError> {
        let account = self.platform_account_repo.get(account_id).await?.ok_or(
            AuthError::TokenExchangeFailed {
                detail: "platform account not found".to_string(),
            },
        )?;
        self.start(
            account.workspace_id,
            account.channel_id,
            account.platform,
            Some(account_id),
            allow_identity_change,
        )
        .await
    }

    async fn start(
        &self,
        workspace_id: Uuid,
        channel_id: Uuid,
        platform: Platform,
        existing_account_id: Option<Uuid>,
        allow_identity_change: bool,
    ) -> Result<Uuid, AuthError> {
        let auth_provider = self.auth_providers.get(&platform).cloned().ok_or(
            AuthError::ProviderNotConfigured {
                detail: format!("{platform} has no auth provider registered"),
            },
        )?;

        // Mark the target account row `Connecting` immediately so the UI
        // reflects it without waiting for a poll round trip.
        if let Some(account_id) = existing_account_id {
            if let Some(mut account) = self.platform_account_repo.get(account_id).await? {
                account.status = PlatformAccountStatus::Connecting;
                account.updated_at = Utc::now();
                let _ = self.platform_account_repo.update(&account).await;
            }
        }

        let session = AuthSession::new(
            platform,
            workspace_id,
            channel_id,
            generate_pkce(),
            String::new(),
        );
        let session_id = session.id;
        let (cancel_tx, cancel_rx) = oneshot::channel();

        self.flights.lock().await.insert(
            session_id,
            Flight {
                cancel: Some(cancel_tx),
                state: AuthFlowState::OpeningBrowser,
            },
        );

        let flights = self.flights.clone();
        let connectors = self.connectors.clone();
        let platform_account_repo = self.platform_account_repo.clone();
        let activity_service = self.activity_service.clone();
        let notification_service = self.notification_service.clone();

        tokio::spawn(async move {
            set_state(&flights, session_id, AuthFlowState::WaitingForAuthorization).await;
            let result = auth_provider.authenticate(session, cancel_rx).await;

            let final_state = match result {
                Ok(identity) => {
                    set_state(&flights, session_id, AuthFlowState::VerifyingAccount).await;
                    match finish_connect(
                        &connectors,
                        &platform_account_repo,
                        &activity_service,
                        &notification_service,
                        workspace_id,
                        channel_id,
                        platform,
                        existing_account_id,
                        allow_identity_change,
                        identity,
                    )
                    .await
                    {
                        Ok(_) => AuthFlowState::Connected,
                        Err(err) => AuthFlowState::Failed {
                            code: err.code().to_string(),
                            message: err.user_message(),
                        },
                    }
                }
                Err(err) => {
                    if let Some(account_id) = existing_account_id {
                        if let Ok(Some(mut account)) = platform_account_repo.get(account_id).await {
                            account.status = PlatformAccountStatus::Error;
                            account.last_error_code = Some(err.code().to_string());
                            account.last_error_message = Some(err.user_message());
                            account.updated_at = Utc::now();
                            let _ = platform_account_repo.update(&account).await;
                        }
                    }
                    AuthFlowState::Failed {
                        code: err.code().to_string(),
                        message: err.user_message(),
                    }
                }
            };
            set_state(&flights, session_id, final_state).await;
            // The flight map has no other eviction path — without this, a
            // long-running session leaks one entry per connect/reconnect
            // attempt for as long as the app stays open. Give the poll
            // loop (800ms interval, section 79) a wide margin to observe
            // the terminal state at least once before dropping it.
            tokio::time::sleep(FLIGHT_RETENTION).await;
            flights.lock().await.remove(&session_id);
        });

        Ok(session_id)
    }

    pub async fn poll(&self, session_id: Uuid) -> Option<AuthFlowState> {
        self.flights
            .lock()
            .await
            .get(&session_id)
            .map(|f| f.state.clone())
    }

    /// Section 45: invalidates the in-flight session — every
    /// `PlatformAuthProvider` implementation observes this promptly and
    /// tears down whatever it opened.
    pub async fn cancel(&self, session_id: Uuid) {
        if let Some(flight) = self.flights.lock().await.get_mut(&session_id) {
            if let Some(cancel) = flight.cancel.take() {
                let _ = cancel.send(());
            }
        }
    }

    pub async fn validate(&self, account_id: Uuid) -> Result<PlatformAccount, AuthError> {
        let account = self.load_account(account_id).await?;
        let connector = self.connector_for(account.platform)?;
        let identity = connector.validate_connection(&account).await?;
        self.apply_identity(&account, identity).await
    }

    pub async fn refresh(&self, account_id: Uuid) -> Result<PlatformAccount, AuthError> {
        // Section 40/154: a true CAS, not a check-then-set — `try_begin_refresh`
        // folds "is a refresh already running" and "mark one running" into
        // a single guarded UPDATE, so two callers racing the periodic
        // sweep against a manual "reconnect"/"validate" click can never
        // both pass the check and both call the provider. Exactly one
        // wins; the other returns here without ever touching the network.
        if !self
            .platform_account_repo
            .try_begin_refresh(account_id)
            .await?
        {
            return Err(AuthError::TokenExchangeFailed {
                detail: "a refresh is already in progress for this account".to_string(),
            });
        }
        let mut account = self.load_account(account_id).await?;

        let connector = self.connector_for(account.platform)?;
        match connector.refresh_connection(&account).await {
            Ok(refreshed) => {
                if let Some(credential) = &refreshed.local_credential {
                    connector
                        .store_local_credential(account.id, credential)
                        .await?;
                }
                account.status = PlatformAccountStatus::Connected;
                account.access_expires_at = refreshed.access_expires_at;
                account.refresh_expires_at = refreshed.refresh_expires_at;
                account.last_refreshed_at = Some(Utc::now());
                account.last_error_code = None;
                account.last_error_message = None;
                account.updated_at = Utc::now();
                self.platform_account_repo.update(&account).await?;
                Ok(account)
            }
            Err(err) => {
                account.status = if matches!(err, AuthError::TokenRevoked) {
                    PlatformAccountStatus::ReauthRequired
                } else {
                    PlatformAccountStatus::Error
                };
                account.last_error_code = Some(err.code().to_string());
                account.last_error_message = Some(err.user_message());
                account.updated_at = Utc::now();
                self.platform_account_repo.update(&account).await?;
                if account.status == PlatformAccountStatus::ReauthRequired {
                    let _ = self
                        .notification_service
                        .notify(
                            NotificationType::Warning,
                            format!("{} needs reauthorization", account.platform.display_name()),
                            format!(
                                "\"{}\" lost its connection and needs to be reconnected.",
                                account.display_name.as_deref().unwrap_or("An account")
                            ),
                        )
                        .await;
                }
                Err(err)
            }
        }
    }

    /// Disconnects an account (section 56/57): best-effort provider-side
    /// revocation, local credential cleanup, and marking the row
    /// `Revoked` — never deleting it, so history and any scheduled
    /// publications referencing it survive untouched.
    pub async fn disconnect(&self, account_id: Uuid) -> Result<PlatformAccount, AuthError> {
        let mut account = self.load_account(account_id).await?;
        if let Ok(connector) = self.connector_for(account.platform) {
            let _ = connector.disconnect(&account).await;
        }
        account.status = PlatformAccountStatus::Revoked;
        account.access_expires_at = None;
        account.refresh_expires_at = None;
        account.updated_at = Utc::now();
        self.platform_account_repo.update(&account).await?;

        self.activity_service
            .log(
                ActivityCategory::Platform,
                ActivityLevel::Warning,
                format!(
                    "{} account \"{}\" disconnected",
                    account.platform.display_name(),
                    account.display_name.as_deref().unwrap_or("unknown")
                ),
            )
            .await
            .ok();
        Ok(account)
    }

    fn connector_for(&self, platform: Platform) -> Result<Arc<dyn PlatformConnector>, AuthError> {
        self.connectors
            .get(&platform)
            .cloned()
            .ok_or(AuthError::ProviderNotConfigured {
                detail: format!("{platform} has no connector registered"),
            })
    }

    async fn load_account(&self, id: Uuid) -> Result<PlatformAccount, AuthError> {
        self.platform_account_repo
            .get(id)
            .await?
            .ok_or(AuthError::TokenExchangeFailed {
                detail: "platform account not found".to_string(),
            })
    }

    /// Re-derives and persists `capabilities`/identity fields from a
    /// fresh `ConnectedIdentity` without touching credential storage or
    /// creating a row — used by `validate`, where the account already
    /// exists and no new token material was issued.
    async fn apply_identity(
        &self,
        account: &PlatformAccount,
        identity: ConnectedIdentity,
    ) -> Result<PlatformAccount, AuthError> {
        let mut account = account.clone();
        account.provider_account_id = Some(identity.provider_account_id);
        account.display_name = identity.display_name.or(account.display_name);
        account.username_or_handle = identity.username_or_handle.or(account.username_or_handle);
        account.avatar_url = identity.avatar_url.or(account.avatar_url);
        if !identity.granted_scopes.is_empty() {
            account.granted_scopes = identity.granted_scopes.clone();
            account.capabilities =
                map_scopes_to_capabilities(account.platform, &identity.granted_scopes);
        }
        account.status = PlatformAccountStatus::Connected;
        account.last_validated_at = Some(Utc::now());
        account.last_error_code = None;
        account.last_error_message = None;
        account.updated_at = Utc::now();

        self.platform_account_repo.update(&account).await?;
        Ok(account)
    }
}

async fn set_state(
    flights: &Arc<Mutex<HashMap<Uuid, Flight>>>,
    session_id: Uuid,
    state: AuthFlowState,
) {
    if let Some(flight) = flights.lock().await.get_mut(&session_id) {
        flight.state = state;
    }
}

#[allow(clippy::too_many_arguments)]
async fn finish_connect(
    connectors: &HashMap<Platform, Arc<dyn PlatformConnector>>,
    platform_account_repo: &Arc<dyn PlatformAccountRepository>,
    activity_service: &Arc<ActivityService>,
    notification_service: &Arc<NotificationService>,
    workspace_id: Uuid,
    channel_id: Uuid,
    platform: Platform,
    mut existing_account_id: Option<Uuid>,
    allow_identity_change: bool,
    identity: ConnectedIdentity,
) -> Result<PlatformAccount, AuthError> {
    // Section 53/59: the same real provider account must not end up
    // *connected* under two different XP FLOW rows in this workspace. A
    // `Revoked` row isn't connected anywhere, though — it's disconnect's
    // "never delete, so history survives" placeholder (see `disconnect`
    // above) — so finding one here for a brand-new connect attempt means
    // "revive this row," not "reject as a duplicate." Without this, a
    // user who disconnects an account and then reconnects it via a fresh
    // "Connect" click (rather than that specific row's "Reconnect"
    // button) would hit a confusing identity-mismatch error against
    // their own prior connection.
    if let Some(owner) = platform_account_repo
        .find_by_provider_identity(workspace_id, platform, &identity.provider_account_id)
        .await?
    {
        if existing_account_id.is_none() && owner.status == PlatformAccountStatus::Revoked {
            existing_account_id = Some(owner.id);
        } else if existing_account_id != Some(owner.id) {
            return Err(AuthError::AccountIdentityMismatch {
                previous_account_id: owner.provider_account_id.clone().unwrap_or_default(),
                new_account_id: identity.provider_account_id.clone(),
            });
        }
    }

    let connector = connectors
        .get(&platform)
        .cloned()
        .ok_or(AuthError::ProviderNotConfigured {
            detail: format!("{platform} has no connector registered"),
        })?;
    if let Some(credential) = &identity.local_credential {
        // Persisted against the *target* account id — for a brand-new
        // connection that id doesn't exist yet, so the credential is
        // stored right after the row is created below instead.
        if let Some(account_id) = existing_account_id {
            connector
                .store_local_credential(account_id, credential)
                .await?;
        }
    }

    let (mut account, is_new) = match existing_account_id {
        Some(id) => {
            let account =
                platform_account_repo
                    .get(id)
                    .await?
                    .ok_or(AuthError::TokenExchangeFailed {
                        detail: "platform account not found".to_string(),
                    })?;
            if let Some(previous) = &account.provider_account_id {
                if previous != &identity.provider_account_id && !allow_identity_change {
                    return Err(AuthError::AccountIdentityMismatch {
                        previous_account_id: previous.clone(),
                        new_account_id: identity.provider_account_id.clone(),
                    });
                }
            }
            (account, false)
        }
        None => (
            PlatformAccount::new(workspace_id, channel_id, platform),
            true,
        ),
    };

    account.provider_account_id = Some(identity.provider_account_id.clone());
    account.display_name = identity.display_name.clone().or(account.display_name);
    account.username_or_handle = identity
        .username_or_handle
        .clone()
        .or(account.username_or_handle);
    account.avatar_url = identity.avatar_url.clone().or(account.avatar_url);
    // Reassigns even a revived row to whatever channel the user picked in
    // this connect attempt — a plain "Reconnect" always targets the same
    // channel it's already on anyway, but a revived-from-Revoked row (see
    // above) must not silently stay pinned to whichever channel it was
    // last connected under.
    account.channel_id = channel_id;
    account.granted_scopes = identity.granted_scopes.clone();
    account.capabilities = map_scopes_to_capabilities(platform, &identity.granted_scopes);
    account.provider_connection_id = identity
        .provider_connection_id
        .clone()
        .or(account.provider_connection_id);
    account.access_expires_at = identity.access_expires_at;
    account.refresh_expires_at = identity.refresh_expires_at;
    account.status = PlatformAccountStatus::Connected;
    account.last_validated_at = Some(Utc::now());
    account.last_error_code = None;
    account.last_error_message = None;
    account.updated_at = Utc::now();
    if is_new {
        account.connected_at = Some(Utc::now());
    }

    if is_new {
        platform_account_repo.create(&account).await?;
        if let Some(credential) = &identity.local_credential {
            connector
                .store_local_credential(account.id, credential)
                .await?;
        }
    } else {
        platform_account_repo.update(&account).await?;
    }

    activity_service
        .log(
            ActivityCategory::Platform,
            ActivityLevel::Success,
            format!(
                "{} account \"{}\" connected",
                platform.display_name(),
                account.display_name.as_deref().unwrap_or("account")
            ),
        )
        .await
        .ok();
    notification_service
        .notify(
            NotificationType::Success,
            format!("{} connected", platform.display_name()),
            format!(
                "\"{}\" is now connected.",
                account.display_name.as_deref().unwrap_or("Your account")
            ),
        )
        .await
        .ok();

    Ok(account)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ports::repositories::{ActivityRepository, NotificationRepository};
    use crate::infrastructure::repositories::{
        SqliteActivityRepository, SqliteNotificationRepository, SqlitePlatformAccountRepository,
    };
    use crate::test_support::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    struct FakeAuthProvider {
        platform: Platform,
        outcome: std::sync::Mutex<Option<Result<ConnectedIdentity, AuthError>>>,
        calls: AtomicUsize,
    }

    impl FakeAuthProvider {
        fn once(platform: Platform, outcome: Result<ConnectedIdentity, AuthError>) -> Arc<Self> {
            Arc::new(Self {
                platform,
                outcome: std::sync::Mutex::new(Some(outcome)),
                calls: AtomicUsize::new(0),
            })
        }
    }

    #[async_trait]
    impl PlatformAuthProvider for FakeAuthProvider {
        fn platform(&self) -> Platform {
            self.platform
        }

        async fn authenticate(
            &self,
            _session: AuthSession,
            mut cancel: oneshot::Receiver<()>,
        ) -> Result<ConnectedIdentity, AuthError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            tokio::select! {
                _ = &mut cancel => Err(AuthError::AuthCancelled),
                outcome = async { self.outcome.lock().unwrap().take().expect("authenticate called more than once on this fake") } => outcome,
            }
        }
    }

    struct SlowCancellableAuthProvider {
        platform: Platform,
    }

    #[async_trait]
    impl PlatformAuthProvider for SlowCancellableAuthProvider {
        fn platform(&self) -> Platform {
            self.platform
        }

        async fn authenticate(
            &self,
            _session: AuthSession,
            mut cancel: oneshot::Receiver<()>,
        ) -> Result<ConnectedIdentity, AuthError> {
            tokio::select! {
                _ = &mut cancel => Err(AuthError::AuthCancelled),
                _ = tokio::time::sleep(Duration::from_secs(30)) => unreachable!("test should cancel long before this"),
            }
        }
    }

    struct FakeConnector {
        platform: Platform,
    }

    #[async_trait]
    impl PlatformConnector for FakeConnector {
        fn platform(&self) -> Platform {
            self.platform
        }
        async fn validate_connection(
            &self,
            _account: &PlatformAccount,
        ) -> Result<ConnectedIdentity, AuthError> {
            unimplemented!("not exercised by these tests")
        }
        async fn refresh_connection(
            &self,
            _account: &PlatformAccount,
        ) -> Result<crate::domain::provider_identity::RefreshedCredentials, AuthError> {
            unimplemented!("not exercised by these tests")
        }
        async fn disconnect(&self, _account: &PlatformAccount) -> Result<(), AuthError> {
            Ok(())
        }
        async fn get_profile(
            &self,
            _account: &PlatformAccount,
        ) -> Result<ConnectedIdentity, AuthError> {
            unimplemented!("not exercised by these tests")
        }
    }

    /// Counts how many times `refresh_connection` was actually invoked,
    /// with a short sleep to widen the race window — proves
    /// `PlatformAuthService::refresh`'s CAS guard, not just the
    /// repository primitive underneath it, actually stops a second
    /// concurrent caller from ever reaching the network.
    struct CountingSlowConnector {
        platform: Platform,
        refresh_calls: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl PlatformConnector for CountingSlowConnector {
        fn platform(&self) -> Platform {
            self.platform
        }
        async fn validate_connection(
            &self,
            _account: &PlatformAccount,
        ) -> Result<ConnectedIdentity, AuthError> {
            unimplemented!("not exercised by this test")
        }
        async fn refresh_connection(
            &self,
            _account: &PlatformAccount,
        ) -> Result<crate::domain::provider_identity::RefreshedCredentials, AuthError> {
            self.refresh_calls.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(20)).await;
            Ok(crate::domain::provider_identity::RefreshedCredentials {
                access_expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
                refresh_expires_at: None,
                local_credential: None,
            })
        }
        async fn disconnect(&self, _account: &PlatformAccount) -> Result<(), AuthError> {
            Ok(())
        }
        async fn get_profile(
            &self,
            _account: &PlatformAccount,
        ) -> Result<ConnectedIdentity, AuthError> {
            unimplemented!("not exercised by this test")
        }
    }

    fn sample_identity(provider_account_id: &str) -> ConnectedIdentity {
        ConnectedIdentity {
            provider_account_id: provider_account_id.to_string(),
            display_name: Some("Football Cuts".to_string()),
            username_or_handle: Some("@footballcuts".to_string()),
            avatar_url: None,
            granted_scopes: vec!["user.info.basic".to_string()],
            access_expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
            refresh_expires_at: None,
            provider_connection_id: Some("conn_123".to_string()),
            local_credential: None,
        }
    }

    async fn build_service(
        pool: sqlx::SqlitePool,
        auth_provider: Arc<dyn PlatformAuthProvider>,
    ) -> (PlatformAuthService, Arc<SqlitePlatformAccountRepository>) {
        build_service_with_connector(
            pool,
            auth_provider,
            Arc::new(FakeConnector {
                platform: Platform::TikTok,
            }),
        )
        .await
    }

    async fn build_service_with_connector(
        pool: sqlx::SqlitePool,
        auth_provider: Arc<dyn PlatformAuthProvider>,
        connector: Arc<dyn PlatformConnector>,
    ) -> (PlatformAuthService, Arc<SqlitePlatformAccountRepository>) {
        let platform_account_repo = Arc::new(SqlitePlatformAccountRepository::new(pool.clone()));
        let activity_repo: Arc<dyn ActivityRepository> =
            Arc::new(SqliteActivityRepository::new(pool.clone()));
        let notification_repo: Arc<dyn NotificationRepository> =
            Arc::new(SqliteNotificationRepository::new(pool));
        let activity_service = Arc::new(ActivityService::new(activity_repo));
        let notification_service = Arc::new(NotificationService::new(notification_repo));

        let mut auth_providers: HashMap<Platform, Arc<dyn PlatformAuthProvider>> = HashMap::new();
        auth_providers.insert(Platform::TikTok, auth_provider);
        let mut connectors: HashMap<Platform, Arc<dyn PlatformConnector>> = HashMap::new();
        connectors.insert(Platform::TikTok, connector);

        let service = PlatformAuthService::new(
            auth_providers,
            connectors,
            platform_account_repo.clone() as Arc<dyn PlatformAccountRepository>,
            activity_service,
            notification_service,
        );
        (service, platform_account_repo)
    }

    async fn wait_for_terminal_state(
        service: &PlatformAuthService,
        session_id: Uuid,
    ) -> AuthFlowState {
        for _ in 0..200 {
            if let Some(state) = service.poll(session_id).await {
                if matches!(
                    state,
                    AuthFlowState::Connected | AuthFlowState::Failed { .. }
                ) {
                    return state;
                }
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("auth flow never reached a terminal state");
    }

    #[tokio::test]
    async fn a_successful_connect_creates_a_connected_account() {
        let pool = temp_pool("auth-svc-connect").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let auth_provider =
            FakeAuthProvider::once(Platform::TikTok, Ok(sample_identity("tiktok-user-1")));
        let (service, repo) = build_service(pool, auth_provider).await;

        let session_id = service
            .begin_connect(workspace_id, channel_id, Platform::TikTok)
            .await
            .unwrap();
        let state = wait_for_terminal_state(&service, session_id).await;
        assert!(matches!(state, AuthFlowState::Connected));

        let accounts = repo.list_for_channel(channel_id).await.unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].status, PlatformAccountStatus::Connected);
        assert_eq!(
            accounts[0].provider_account_id.as_deref(),
            Some("tiktok-user-1")
        );
        assert!(accounts[0]
            .capabilities
            .contains(&crate::domain::capability::Capability::ReadProfile));
    }

    #[tokio::test]
    async fn connecting_the_same_real_account_to_two_channels_is_rejected() {
        let pool = temp_pool("auth-svc-dup").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let channel_a = seed_channel(&pool, workspace_id, "Channel A").await;
        let channel_b = seed_channel(&pool, workspace_id, "Channel B").await;

        let first_provider =
            FakeAuthProvider::once(Platform::TikTok, Ok(sample_identity("tiktok-user-shared")));
        let (service_a, _repo) = build_service(pool.clone(), first_provider).await;
        let session_a = service_a
            .begin_connect(workspace_id, channel_a, Platform::TikTok)
            .await
            .unwrap();
        assert!(matches!(
            wait_for_terminal_state(&service_a, session_a).await,
            AuthFlowState::Connected
        ));

        let second_provider =
            FakeAuthProvider::once(Platform::TikTok, Ok(sample_identity("tiktok-user-shared")));
        let (service_b, _repo) = build_service(pool, second_provider).await;
        let session_b = service_b
            .begin_connect(workspace_id, channel_b, Platform::TikTok)
            .await
            .unwrap();
        let state = wait_for_terminal_state(&service_b, session_b).await;
        assert!(
            matches!(state, AuthFlowState::Failed { code, .. } if code == "ACCOUNT_IDENTITY_MISMATCH")
        );
    }

    #[tokio::test]
    async fn reconnecting_the_same_real_account_after_disconnect_revives_the_row_on_the_new_channel(
    ) {
        let pool = temp_pool("auth-svc-revive").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let channel_a = seed_channel(&pool, workspace_id, "Channel A").await;
        let channel_b = seed_channel(&pool, workspace_id, "Channel B").await;

        let first_provider =
            FakeAuthProvider::once(Platform::TikTok, Ok(sample_identity("tiktok-user-revived")));
        let (service, repo) = build_service(pool.clone(), first_provider).await;
        let session_id = service
            .begin_connect(workspace_id, channel_a, Platform::TikTok)
            .await
            .unwrap();
        assert!(matches!(
            wait_for_terminal_state(&service, session_id).await,
            AuthFlowState::Connected
        ));
        let original_id = repo.list_for_channel(channel_a).await.unwrap()[0].id;

        service.disconnect(original_id).await.unwrap();

        // A brand-new "Connect" (not that row's own "Reconnect") for the
        // exact same real account, now targeting a different channel.
        let second_provider =
            FakeAuthProvider::once(Platform::TikTok, Ok(sample_identity("tiktok-user-revived")));
        let (service_b, _repo) = build_service(pool, second_provider).await;
        let session_b = service_b
            .begin_connect(workspace_id, channel_b, Platform::TikTok)
            .await
            .unwrap();
        let state = wait_for_terminal_state(&service_b, session_b).await;
        assert!(matches!(state, AuthFlowState::Connected));

        let accounts_b = repo.list_for_channel(channel_b).await.unwrap();
        assert_eq!(
            accounts_b.len(),
            1,
            "the revoked row should move here, not duplicate"
        );
        assert_eq!(accounts_b[0].id, original_id, "same row, revived in place");
        assert_eq!(accounts_b[0].status, PlatformAccountStatus::Connected);

        assert!(
            repo.list_for_channel(channel_a).await.unwrap().is_empty(),
            "the row no longer belongs to its original channel"
        );
    }

    #[tokio::test]
    async fn reconnecting_with_a_different_real_account_is_rejected_unless_forced() {
        let pool = temp_pool("auth-svc-reconnect-mismatch").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;

        let first_provider = FakeAuthProvider::once(
            Platform::TikTok,
            Ok(sample_identity("tiktok-user-original")),
        );
        let (service, repo) = build_service(pool.clone(), first_provider).await;
        let session_id = service
            .begin_connect(workspace_id, channel_id, Platform::TikTok)
            .await
            .unwrap();
        assert!(matches!(
            wait_for_terminal_state(&service, session_id).await,
            AuthFlowState::Connected
        ));
        let account_id = repo.list_for_channel(channel_id).await.unwrap()[0].id;

        // Reconnect with a *different* account, not forced — must fail closed.
        let mismatched_provider = FakeAuthProvider::once(
            Platform::TikTok,
            Ok(sample_identity("tiktok-user-different")),
        );
        let (service2, repo2) = build_service(pool, mismatched_provider).await;
        let session2 = service2.begin_reconnect(account_id, false).await.unwrap();
        let state = wait_for_terminal_state(&service2, session2).await;
        assert!(
            matches!(state, AuthFlowState::Failed { code, .. } if code == "ACCOUNT_IDENTITY_MISMATCH")
        );

        // The original identity must be untouched.
        let account = repo2.get(account_id).await.unwrap().unwrap();
        assert_eq!(
            account.provider_account_id.as_deref(),
            Some("tiktok-user-original")
        );
    }

    #[tokio::test]
    async fn cancelling_an_in_flight_connect_reports_cancelled() {
        let pool = temp_pool("auth-svc-cancel").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let provider = Arc::new(SlowCancellableAuthProvider {
            platform: Platform::TikTok,
        });
        let (service, _repo) = build_service(pool, provider).await;

        let session_id = service
            .begin_connect(workspace_id, channel_id, Platform::TikTok)
            .await
            .unwrap();
        // Give the spawned task a moment to actually start waiting.
        tokio::time::sleep(Duration::from_millis(20)).await;
        service.cancel(session_id).await;

        let state = wait_for_terminal_state(&service, session_id).await;
        assert!(matches!(state, AuthFlowState::Failed { code, .. } if code == "AUTH_CANCELLED"));
    }

    #[tokio::test]
    async fn a_failed_authorization_never_creates_an_account_row() {
        let pool = temp_pool("auth-svc-failure").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let provider = FakeAuthProvider::once(Platform::TikTok, Err(AuthError::PermissionDenied));
        let (service, repo) = build_service(pool, provider).await;

        let session_id = service
            .begin_connect(workspace_id, channel_id, Platform::TikTok)
            .await
            .unwrap();
        let state = wait_for_terminal_state(&service, session_id).await;
        assert!(matches!(state, AuthFlowState::Failed { code, .. } if code == "PERMISSION_DENIED"));
        assert!(repo.list_for_channel(channel_id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn two_concurrent_refresh_calls_only_reach_the_provider_once() {
        let pool = temp_pool("auth-svc-refresh-cas").await;
        let (workspace_id, _source_id) = seed_workspace_and_source(&pool).await;
        let channel_id = seed_channel(&pool, workspace_id, "Main").await;
        let provider =
            FakeAuthProvider::once(Platform::TikTok, Ok(sample_identity("tiktok-refresh-race")));
        let refresh_calls = Arc::new(AtomicUsize::new(0));
        let (service, repo) = build_service_with_connector(
            pool,
            provider,
            Arc::new(CountingSlowConnector {
                platform: Platform::TikTok,
                refresh_calls: refresh_calls.clone(),
            }),
        )
        .await;

        // Connect once so a real account row exists to refresh.
        let session_id = service
            .begin_connect(workspace_id, channel_id, Platform::TikTok)
            .await
            .unwrap();
        assert!(matches!(
            wait_for_terminal_state(&service, session_id).await,
            AuthFlowState::Connected
        ));
        let account_id = repo.list_for_channel(channel_id).await.unwrap()[0].id;

        let service = Arc::new(service);
        let (a, b) = tokio::join!(
            {
                let service = service.clone();
                async move { service.refresh(account_id).await }
            },
            {
                let service = service.clone();
                async move { service.refresh(account_id).await }
            }
        );
        let successes = [a.is_ok(), b.is_ok()].into_iter().filter(|ok| *ok).count();

        assert_eq!(
            successes, 1,
            "exactly one concurrent refresh should succeed"
        );
        assert_eq!(
            refresh_calls.load(Ordering::SeqCst),
            1,
            "the losing caller must never reach the provider at all"
        );
    }
}
