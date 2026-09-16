use async_trait::async_trait;

use crate::domain::platform::Platform;
use crate::domain::platform_account::PlatformAccount;
use crate::domain::ports::platform_publisher::{CancelSignal, PlatformPublisher, ProgressSender};
use crate::domain::publishing::{PublishError, RemoteUploadState, RenderedMetadata, UploadSession};
use crate::domain::video::Video;

/// The `PlatformPublisher` counterpart to
/// `infrastructure::connectors::StubConnector` — wired in for any
/// platform whose real uploader isn't configured (or not yet
/// implemented), so a claimed publication fails with a clear,
/// non-retryable `PlatformNotApproved` instead of the app crashing or
/// silently doing nothing.
pub struct StubPublisher {
    platform: Platform,
    reason: String,
}

impl StubPublisher {
    pub fn new(platform: Platform, reason: impl Into<String>) -> Self {
        Self {
            platform,
            reason: reason.into(),
        }
    }

    fn err(&self) -> PublishError {
        PublishError::PlatformNotApproved {
            detail: self.reason.clone(),
        }
    }
}

#[async_trait]
impl PlatformPublisher for StubPublisher {
    fn platform(&self) -> Platform {
        self.platform
    }

    async fn validate_media(&self, _video: &Video) -> Result<(), PublishError> {
        Err(self.err())
    }

    fn validate_metadata(&self, _metadata: &RenderedMetadata) -> Result<(), PublishError> {
        Err(self.err())
    }

    async fn initialize_upload(
        &self,
        _account: &PlatformAccount,
        _access_token: &str,
        _video: &Video,
        _metadata: &RenderedMetadata,
    ) -> Result<UploadSession, PublishError> {
        Err(self.err())
    }

    async fn upload_media(
        &self,
        _access_token: &str,
        session: UploadSession,
        _video: &Video,
        _progress: ProgressSender,
        _cancel: CancelSignal,
    ) -> (UploadSession, Result<(), PublishError>) {
        (session, Err(self.err()))
    }

    async fn finalize_publication(
        &self,
        _access_token: &str,
        _session: UploadSession,
    ) -> Result<UploadSession, PublishError> {
        Err(self.err())
    }

    async fn get_remote_status(
        &self,
        _access_token: &str,
        _session: &UploadSession,
    ) -> Result<RemoteUploadState, PublishError> {
        Err(self.err())
    }

    async fn recover_upload(
        &self,
        _access_token: &str,
        _session: UploadSession,
        _video: &Video,
    ) -> Result<UploadSession, PublishError> {
        Err(self.err())
    }

    async fn cancel_upload_if_supported(
        &self,
        _access_token: &str,
        _session: &UploadSession,
    ) -> Result<(), PublishError> {
        Ok(())
    }
}
