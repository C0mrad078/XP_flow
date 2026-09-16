use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::platform::Platform;
use crate::domain::platform_account::PlatformAccount;
use crate::domain::ports::platform_publisher::{
    CancelSignal, PlatformPublisher, ProgressSender, UploadProgress,
};
use crate::domain::publishing::{
    PublishError, RemoteUploadState, RenderedMetadata, SessionType, UploadSession,
};
use crate::domain::video::Video;

/// A scripted outcome for [`FakePublisher`] — first-class, deterministic
/// provider simulation (section 142/161) so engine correctness (claiming,
/// attempts, crash recovery, retries) is fully testable without live
/// credentials, and so a future dev-only "simulate a publish" UI has a
/// real backend to drive.
#[derive(Debug, Clone)]
pub enum FakeScenario {
    /// Uploads and finalizes cleanly in one pass.
    Success,
    /// Fails before any bytes are sent — `initialize_upload` itself
    /// returns the given error. Always safe to restart.
    FailBeforeUpload(PublishError),
    /// Simulates a network drop partway through the transfer: the first
    /// `upload_media` call commits half the bytes then fails with
    /// `NetworkTransient`, leaving the session `Transferring` (*not*
    /// safe to restart). A subsequent `recover_upload` call resumes from
    /// the committed bytes and completes successfully — this is the
    /// scenario the crash-recovery tests exercise.
    InterruptedThenRecoverable,
    /// The provider needs asynchronous processing after transfer
    /// completes: `finalize_publication` returns `RemoteProcessing`, and
    /// `get_remote_status` only reports `RemoteSucceeded` after being
    /// called `processing_polls_until_done` times.
    NeedsProcessing { polls_until_done: u32 },
}

struct FakeState {
    scenario: FakeScenario,
    poll_count: u32,
}

/// A fully scripted [`PlatformPublisher`] — never touches a network.
/// `platform` only affects what gets stamped on the session/metadata; the
/// actual behavior comes entirely from `scenario`.
pub struct FakePublisher {
    platform: Platform,
    state: Mutex<FakeState>,
    cancelled: AtomicBool,
}

impl FakePublisher {
    pub fn new(platform: Platform, scenario: FakeScenario) -> Self {
        Self {
            platform,
            state: Mutex::new(FakeState {
                scenario,
                poll_count: 0,
            }),
            cancelled: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl PlatformPublisher for FakePublisher {
    fn platform(&self) -> Platform {
        self.platform
    }

    async fn validate_media(&self, _video: &Video) -> Result<(), PublishError> {
        Ok(())
    }

    fn validate_metadata(&self, metadata: &RenderedMetadata) -> Result<(), PublishError> {
        if metadata.title.trim().is_empty() {
            return Err(PublishError::InvalidMetadata {
                detail: "title cannot be empty".to_string(),
            });
        }
        Ok(())
    }

    async fn initialize_upload(
        &self,
        _account: &PlatformAccount,
        _access_token: &str,
        video: &Video,
        _metadata: &RenderedMetadata,
    ) -> Result<UploadSession, PublishError> {
        let scenario = self.state.lock().unwrap().scenario.clone();
        if let FakeScenario::FailBeforeUpload(err) = scenario {
            return Err(err);
        }
        let mut session = UploadSession::new(
            Uuid::nil(),
            Uuid::nil(),
            self.platform,
            SessionType::Resumable,
        );
        session.bytes_total = Some(video.file_size_bytes);
        session.remote_session_id = Some(format!("fake-session-{}", Uuid::new_v4()));
        session.state = RemoteUploadState::Initialized;
        Ok(session)
    }

    async fn upload_media(
        &self,
        _access_token: &str,
        mut session: UploadSession,
        video: &Video,
        progress: ProgressSender,
        cancel: CancelSignal,
    ) -> Result<UploadSession, PublishError> {
        let scenario = self.state.lock().unwrap().scenario.clone();
        let total = video.file_size_bytes;

        if cancel.is_cancelled() {
            return Err(PublishError::Cancelled);
        }

        match scenario {
            FakeScenario::InterruptedThenRecoverable if session.bytes_committed == 0 => {
                let half = total / 2;
                session.bytes_committed = half;
                session.state = RemoteUploadState::Transferring;
                let _ = progress.send(UploadProgress {
                    bytes_uploaded: half,
                    bytes_total: Some(total),
                });
                Err(PublishError::NetworkTransient)
            }
            _ => {
                session.bytes_committed = total;
                session.state = RemoteUploadState::Transferred;
                let _ = progress.send(UploadProgress {
                    bytes_uploaded: total,
                    bytes_total: Some(total),
                });
                Ok(session)
            }
        }
    }

    async fn finalize_publication(
        &self,
        _access_token: &str,
        mut session: UploadSession,
    ) -> Result<UploadSession, PublishError> {
        let scenario = self.state.lock().unwrap().scenario.clone();
        match scenario {
            FakeScenario::NeedsProcessing { .. } => {
                session.state = RemoteUploadState::RemoteProcessing;
                session.remote_publish_id = Some(format!("fake-remote-{}", Uuid::new_v4()));
            }
            _ => {
                session.state = RemoteUploadState::RemoteSucceeded;
                session.remote_publish_id = Some(format!("fake-remote-{}", Uuid::new_v4()));
            }
        }
        session.updated_at = Utc::now();
        Ok(session)
    }

    async fn get_remote_status(
        &self,
        _access_token: &str,
        session: &UploadSession,
    ) -> Result<RemoteUploadState, PublishError> {
        let mut guard = self.state.lock().unwrap();
        let polls_until_done = match &guard.scenario {
            FakeScenario::NeedsProcessing { polls_until_done } => Some(*polls_until_done),
            _ => None,
        };
        match polls_until_done {
            Some(polls_until_done) => {
                guard.poll_count += 1;
                if guard.poll_count >= polls_until_done {
                    Ok(RemoteUploadState::RemoteSucceeded)
                } else {
                    Ok(RemoteUploadState::RemoteProcessing)
                }
            }
            None => Ok(session.state),
        }
    }

    async fn recover_upload(
        &self,
        access_token: &str,
        session: UploadSession,
    ) -> Result<UploadSession, PublishError> {
        if session.state.safe_to_restart() {
            return Ok(session);
        }
        // Simulates verifying with the provider and finding the transfer
        // can resume from the committed byte count — a real provider's
        // recovery would call a status/session-query endpoint here
        // instead of blindly assuming success.
        let (_sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let video = Video::new(
            Uuid::nil(),
            Uuid::nil(),
            None,
            "recovered".to_string(),
            "recovered",
            "/dev/null".to_string(),
            session.bytes_total.unwrap_or(0),
            "mp4",
        );
        self.upload_media(access_token, session, &video, _sender, CancelSignal::new())
            .await
    }

    async fn cancel_upload_if_supported(
        &self,
        _access_token: &str,
        _session: &UploadSession,
    ) -> Result<(), PublishError> {
        self.cancelled.store(true, Ordering::SeqCst);
        Ok(())
    }
}
