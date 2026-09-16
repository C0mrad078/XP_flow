use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::platform::Platform;
use crate::domain::platform_account::PlatformAccount;
use crate::domain::publishing::{PublishError, RemoteUploadState, RenderedMetadata, UploadSession};
use crate::domain::video::Video;

/// Live upload progress, pushed by the uploader as bytes are
/// acknowledged by the provider — never invented, always the actual
/// committed byte count (section 53). The `PublishingEngineService`
/// forwards these to the frontend event bus (section 90) and only
/// persists a durable checkpoint at sensible intervals (section 149),
/// not on every event.
#[derive(Debug, Clone, Copy)]
pub struct UploadProgress {
    pub bytes_uploaded: i64,
    pub bytes_total: Option<i64>,
}

pub type ProgressSender = tokio::sync::mpsc::UnboundedSender<UploadProgress>;

/// A cooperative cancel signal checked between chunks/steps, never
/// mid-write (section 127: "cancel local future work" is safe at any
/// boundary; a byte range already in flight to the provider is allowed
/// to finish rather than leaving a half-sent chunk).
#[derive(Clone, Default)]
pub struct CancelSignal(Arc<AtomicBool>);

impl CancelSignal {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// The upgraded, provider-neutral publishing contract (section 6) —
/// domain intent, not provider-specific HTTP details. Each method's
/// exact provider behavior lives entirely inside the implementation;
/// this trait only fixes the *shape* every provider is expected to
/// expose so `PublishingEngineService` never branches on `Platform`.
///
/// Distinct from `domain::ports::platform_connector::PlatformConnector`,
/// which governs account-lifecycle operations (validate/refresh/
/// disconnect) that have nothing to do with uploading media — the same
/// two-port split Phase 4 established between `PlatformAuthProvider` and
/// `PlatformConnector` (section 4).
#[async_trait]
pub trait PlatformPublisher: Send + Sync {
    fn platform(&self) -> Platform;

    /// Local, offline checks only (file exists, format/size/duration
    /// constraints this provider documents) — no network call.
    async fn validate_media(&self, video: &Video) -> Result<(), PublishError>;

    /// This provider's current metadata constraints (section 29) — title
    /// length, caption length, provider-option value ranges. Pure, no I/O.
    fn validate_metadata(&self, metadata: &RenderedMetadata) -> Result<(), PublishError>;

    /// Starts a brand-new upload session against the provider. Must only
    /// be called when no prior session exists or the prior one is
    /// `safe_to_restart` (section 3/66) — the caller (`PublishingEngineService`)
    /// owns that decision, not the publisher.
    async fn initialize_upload(
        &self,
        account: &PlatformAccount,
        access_token: &str,
        video: &Video,
        metadata: &RenderedMetadata,
    ) -> Result<UploadSession, PublishError>;

    /// Streams `video`'s file per this provider's transport (resumable /
    /// chunked / stepwise — section 41: always from disk, never buffering
    /// the whole file in memory), reporting acknowledged progress through
    /// `progress`. Checks `cancel` between chunks. Returns the session
    /// with updated `bytes_committed`/`state`.
    async fn upload_media(
        &self,
        access_token: &str,
        session: UploadSession,
        video: &Video,
        progress: ProgressSender,
        cancel: CancelSignal,
    ) -> Result<UploadSession, PublishError>;

    /// The provider-specific "make it actually post" step where one
    /// exists (TikTok/Kwai's explicit publish call). A no-op passthrough
    /// is correct for a provider whose transfer-completion response is
    /// already the finalized result (YouTube).
    async fn finalize_publication(
        &self,
        access_token: &str,
        session: UploadSession,
    ) -> Result<UploadSession, PublishError>;

    /// Polls the provider for the current remote processing state —
    /// never assumes "transferred" implies "publicly available" (section
    /// 45/57).
    async fn get_remote_status(
        &self,
        access_token: &str,
        session: &UploadSession,
    ) -> Result<RemoteUploadState, PublishError>;

    /// Given a persisted, possibly-stale session (loaded after a crash or
    /// an ambiguous failure), determines whether it's safe to resume,
    /// must be verified against the provider first, or must be
    /// discarded — never blindly starts a new session over one whose
    /// `RemoteUploadState` isn't `safe_to_restart` (section 42/43/55/88).
    async fn recover_upload(
        &self,
        access_token: &str,
        session: UploadSession,
    ) -> Result<UploadSession, PublishError>;

    /// Best-effort cancellation of an in-progress (not yet finalized)
    /// upload where the provider exposes one. Must never delete an
    /// already-published remote post — that is explicitly out of scope
    /// for Phase 5 (section 127/128).
    async fn cancel_upload_if_supported(
        &self,
        access_token: &str,
        session: &UploadSession,
    ) -> Result<(), PublishError>;
}
