use std::io::SeekFrom;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
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
use crate::infrastructure::publishing::http_client::{
    build_upload_http_client, parse_retry_after_seconds, PROCESSING_STATUS_TIMEOUT,
    UPLOAD_CHUNK_TIMEOUT, UPLOAD_CONTROL_TIMEOUT,
};

use super::config::KwaiPublishConfig;

const DEFAULT_BASE_URL: &str = "https://open.kuaishou.com";

const START_UPLOAD_SCOPE: &str = "openapi/photo/start_upload";
const PUBLISH_SCOPE: &str = "openapi/photo/publish";
const LIST_SCOPE: &str = "openapi/photo/list";
const FRAGMENT_SCOPE: &str = "api/upload/fragment";
const RESUME_SCOPE: &str = "api/upload/resume";
const COMPLETE_SCOPE: &str = "api/upload/complete";

/// IMPORTANT — honesty note (section 58-61/164): Kwai's own developer
/// documentation was not reachable from this environment. This uploader
/// is grounded instead in the request/response shapes of an unofficial,
/// third-party Go client (`github.com/bububa/kwai-openapi`), which wraps
/// `open.kuaishou.com` (Kwai and Kuaishou share the same open platform).
/// Anything not directly observable in that source — exact per-fragment
/// size limits, the full `result` error-code table, and caption length
/// limits — is a conservative, explicitly-labeled assumption below, not
/// a verified provider constraint. See `docs/kwai-publishing.md`.
///
/// No official size/count constraint for a single fragment was found;
/// this value is a conservative default chosen to keep memory use and
/// per-request duration reasonable, not a documented Kwai requirement.
const FRAGMENT_SIZE: i64 = 4 * 1024 * 1024;

struct Endpoints {
    base: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            base: DEFAULT_BASE_URL.to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct BaseResult {
    result: i32,
    #[serde(default)]
    error_msg: String,
}

#[derive(Debug, Deserialize)]
struct StartUploadResponse {
    #[serde(flatten)]
    base: BaseResult,
    #[serde(default)]
    upload_token: String,
    #[serde(default)]
    endpoint: String,
}

#[derive(Debug, Deserialize)]
struct ResumeResponse {
    #[serde(flatten)]
    base: BaseResult,
    #[serde(default)]
    existed: bool,
    #[serde(default)]
    fragment_index_bytes: i64,
}

#[derive(Debug, Deserialize)]
struct UserVideo {
    #[serde(default)]
    photo_id: String,
    #[serde(default)]
    pending: bool,
}

#[derive(Debug, Deserialize)]
struct PublishResponse {
    #[serde(flatten)]
    base: BaseResult,
    #[serde(default)]
    video_info: Option<UserVideo>,
}

#[derive(Debug, Deserialize)]
struct ListResponse {
    #[serde(flatten)]
    base: BaseResult,
    #[serde(default)]
    video_list: Vec<UserVideo>,
}

/// Kwai's real stepwise upload `PlatformPublisher` (section 58-61):
/// start_upload → chunked fragment transfer → complete → publish, with
/// status checked through the account's video list (the only status
/// surface this unofficial API exposes — see `get_remote_status` below).
pub struct KwaiUploader {
    http: reqwest::Client,
    endpoints: Endpoints,
    app_id: String,
}

impl KwaiUploader {
    pub fn new(config: KwaiPublishConfig) -> Self {
        Self {
            http: build_upload_http_client(),
            endpoints: Endpoints::default(),
            app_id: config.app_id,
        }
    }

    #[cfg(test)]
    fn with_base_url(base_url: &str, app_id: &str) -> Self {
        Self {
            http: build_upload_http_client(),
            endpoints: Endpoints {
                base: base_url.to_string(),
            },
            app_id: app_id.to_string(),
        }
    }

    async fn start_upload(&self, access_token: &str) -> Result<StartUploadResponse, PublishError> {
        let response = self
            .http
            .post(format!("{}/{START_UPLOAD_SCOPE}", self.endpoints.base))
            .query(&[
                ("app_id", self.app_id.as_str()),
                ("access_token", access_token),
            ])
            .timeout(UPLOAD_CONTROL_TIMEOUT)
            .send()
            .await
            .map_err(map_transport_err)?;
        let retry_after_seconds = parse_retry_after_seconds(response.headers());
        let body: StartUploadResponse =
            response.json().await.map_err(|e| PublishError::Internal {
                detail: e.to_string(),
            })?;
        if body.base.result != 1 {
            return Err(classify_result(&body.base.error_msg, retry_after_seconds));
        }
        Ok(body)
    }
}

fn classify_result(error_msg: &str, retry_after_seconds: Option<u64>) -> PublishError {
    // No official error-code table was available (see the module-level
    // honesty note) — this is best-effort substring matching on the
    // human-readable message, not a verified mapping.
    let lower = error_msg.to_lowercase();
    if lower.contains("token") && (lower.contains("expire") || lower.contains("invalid")) {
        PublishError::AuthExpired
    } else if lower.contains("frequen") || lower.contains("rate") || lower.contains("limit") {
        PublishError::RateLimited {
            retry_after_seconds,
        }
    } else if lower.contains("permission") || lower.contains("scope") {
        PublishError::PermissionMissing {
            capability: "upload_video".to_string(),
        }
    } else {
        PublishError::Internal {
            detail: format!("Kwai returned: {error_msg}"),
        }
    }
}

fn map_transport_err(err: reqwest::Error) -> PublishError {
    if err.is_timeout() || err.is_connect() {
        PublishError::NetworkTransient
    } else {
        PublishError::Internal {
            detail: err.to_string(),
        }
    }
}

fn fragment_count(total: i64) -> i64 {
    (total + FRAGMENT_SIZE - 1) / FRAGMENT_SIZE
}

#[async_trait]
impl PlatformPublisher for KwaiUploader {
    fn platform(&self) -> Platform {
        Platform::Kwai
    }

    async fn validate_media(&self, video: &Video) -> Result<(), PublishError> {
        if video.file_size_bytes <= 0 {
            return Err(PublishError::InvalidMedia {
                detail: "the video file is empty".to_string(),
            });
        }
        // No documented Kwai size cap was found in the available source
        // — this is a generous sanity bound, not a verified limit.
        const GENEROUS_MAX_BYTES: i64 = 4 * 1024 * 1024 * 1024;
        if video.file_size_bytes > GENEROUS_MAX_BYTES {
            return Err(PublishError::InvalidMedia {
                detail: "the video file exceeds a conservative upload size sanity bound"
                    .to_string(),
            });
        }
        Ok(())
    }

    fn validate_metadata(&self, metadata: &RenderedMetadata) -> Result<(), PublishError> {
        if metadata.title.trim().is_empty() {
            return Err(PublishError::InvalidMetadata {
                detail: "caption cannot be empty".to_string(),
            });
        }
        Ok(())
    }

    async fn initialize_upload(
        &self,
        _account: &PlatformAccount,
        access_token: &str,
        video: &Video,
        _metadata: &RenderedMetadata,
    ) -> Result<UploadSession, PublishError> {
        let started = self.start_upload(access_token).await?;
        if started.upload_token.is_empty() || started.endpoint.is_empty() {
            return Err(PublishError::Internal {
                detail: "Kwai start_upload returned no upload_token/endpoint".to_string(),
            });
        }

        let mut session = UploadSession::new(
            Uuid::nil(),
            Uuid::nil(),
            Platform::Kwai,
            SessionType::Stepwise,
        );
        session.remote_session_id = Some(started.upload_token);
        session.remote_upload_url = Some(format!("https://{}", started.endpoint));
        session.bytes_total = Some(video.file_size_bytes);
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
    ) -> (UploadSession, Result<(), PublishError>) {
        let (Some(upload_host), Some(upload_token)) = (
            session.remote_upload_url.clone(),
            session.remote_session_id.clone(),
        ) else {
            return (
                session,
                Err(PublishError::Internal {
                    detail: "no upload host/token on session".to_string(),
                }),
            );
        };
        let total = session.bytes_total.unwrap_or(video.file_size_bytes);

        let mut file = match tokio::fs::File::open(&video.file_path).await {
            Ok(file) => file,
            Err(_) => return (session, Err(PublishError::VideoUnavailable)),
        };

        let mut offset = session.bytes_committed;
        while offset < total {
            if cancel.is_cancelled() {
                return (session, Err(PublishError::Cancelled));
            }
            let this_chunk = (total - offset).min(FRAGMENT_SIZE);
            let fragment_id = offset / FRAGMENT_SIZE;

            if file.seek(SeekFrom::Start(offset as u64)).await.is_err() {
                return (session, Err(PublishError::VideoUnavailable));
            }
            let mut buf = vec![0u8; this_chunk as usize];
            if file.read_exact(&mut buf).await.is_err() {
                return (session, Err(PublishError::VideoUnavailable));
            }

            let result = self
                .http
                .post(format!("{upload_host}/{FRAGMENT_SCOPE}"))
                .query(&[
                    ("upload_token", upload_token.as_str()),
                    ("fragment_id", fragment_id.to_string().as_str()),
                ])
                .header("Content-Type", "application/octet-stream")
                .body(buf)
                .timeout(UPLOAD_CHUNK_TIMEOUT)
                .send()
                .await;

            let response = match result {
                Ok(response) => response,
                Err(err) => {
                    session.state = RemoteUploadState::Transferring;
                    return (session, Err(map_transport_err(err)));
                }
            };

            let retry_after_seconds = parse_retry_after_seconds(response.headers());
            let body: BaseResult = match response.json().await {
                Ok(body) => body,
                Err(_) => {
                    session.state = RemoteUploadState::RemoteUnknown;
                    return (session, Err(PublishError::UnknownRemoteResult));
                }
            };
            if body.result != 1 {
                session.state = RemoteUploadState::Transferring;
                return (
                    session,
                    Err(classify_result(&body.error_msg, retry_after_seconds)),
                );
            }

            offset += this_chunk;
            session.bytes_committed = offset;
            let _ = progress.send(UploadProgress {
                bytes_uploaded: offset,
                bytes_total: Some(total),
            });
        }

        session.state = RemoteUploadState::Transferred;
        (session, Ok(()))
    }

    async fn finalize_publication(
        &self,
        access_token: &str,
        mut session: UploadSession,
    ) -> Result<UploadSession, PublishError> {
        let (Some(upload_host), Some(upload_token)) = (
            session.remote_upload_url.clone(),
            session.remote_session_id.clone(),
        ) else {
            return Err(PublishError::Internal {
                detail: "no upload host/token on session".to_string(),
            });
        };
        let total = session.bytes_total.unwrap_or(0);

        let complete_response = self
            .http
            .post(format!("{upload_host}/{COMPLETE_SCOPE}"))
            .query(&[
                ("upload_token", upload_token.as_str()),
                ("fragment_count", fragment_count(total).to_string().as_str()),
            ])
            .timeout(UPLOAD_CONTROL_TIMEOUT)
            .send()
            .await
            .map_err(map_transport_err)?;
        let complete_retry_after_seconds = parse_retry_after_seconds(complete_response.headers());
        let complete_body: BaseResult =
            complete_response
                .json()
                .await
                .map_err(|e| PublishError::Internal {
                    detail: e.to_string(),
                })?;
        if complete_body.result != 1 {
            // Nothing published yet — safe to surface as a normal,
            // classifiable error and let the caller retry finalize.
            return Err(classify_result(
                &complete_body.error_msg,
                complete_retry_after_seconds,
            ));
        }

        // The publish call is the exactly-once-critical remote write
        // (section 3): unlike TikTok's init, no post exists until this
        // succeeds. A transport-level failure here means the outcome is
        // genuinely unknown — Kwai may have already registered the post
        // — so this must never be silently retried (section 69).
        let form = reqwest::multipart::Form::new()
            .text("cover", "")
            .text("caption", "")
            .text("stero_type", "NOT_SPHERICAL_VIDEO");
        let publish_result = self
            .http
            .post(format!("{}/{PUBLISH_SCOPE}", self.endpoints.base))
            .query(&[
                ("app_id", self.app_id.as_str()),
                ("access_token", access_token),
                ("upload_token", upload_token.as_str()),
            ])
            .multipart(form)
            .timeout(UPLOAD_CONTROL_TIMEOUT)
            .send()
            .await;

        let publish_response = match publish_result {
            Ok(response) => response,
            Err(_) => {
                session.state = RemoteUploadState::RemoteUnknown;
                return Err(PublishError::UnknownRemoteResult);
            }
        };
        let publish_retry_after_seconds = parse_retry_after_seconds(publish_response.headers());
        let publish_body: PublishResponse = match publish_response.json().await {
            Ok(body) => body,
            Err(_) => {
                session.state = RemoteUploadState::RemoteUnknown;
                return Err(PublishError::UnknownRemoteResult);
            }
        };
        if publish_body.base.result != 1 {
            // A clean error response from Kwai — no post was created,
            // this is unambiguous and safe to classify normally.
            return Err(classify_result(
                &publish_body.base.error_msg,
                publish_retry_after_seconds,
            ));
        }
        let Some(video_info) = publish_body.video_info else {
            session.state = RemoteUploadState::RemoteUnknown;
            return Err(PublishError::UnknownRemoteResult);
        };

        session.remote_publish_id = Some(video_info.photo_id);
        session.state = if video_info.pending {
            RemoteUploadState::RemoteProcessing
        } else {
            RemoteUploadState::RemoteSucceeded
        };
        Ok(session)
    }

    async fn get_remote_status(
        &self,
        access_token: &str,
        session: &UploadSession,
    ) -> Result<RemoteUploadState, PublishError> {
        let Some(photo_id) = &session.remote_publish_id else {
            // No confirmed post id — this unofficial API exposes no way
            // to look up "did upload_token X ever get published"
            // (section 58-61's documented gap). The last known state is
            // the most honest answer available.
            return Ok(session.state);
        };

        // The only status surface this API exposes is the paginated
        // video list — bounded scan, never an unbounded search.
        let mut cursor = String::new();
        for _ in 0..3 {
            let mut query = vec![
                ("app_id", self.app_id.as_str()),
                ("access_token", access_token),
                ("count", "50"),
            ];
            if !cursor.is_empty() {
                query.push(("cursor", cursor.as_str()));
            }
            let response = self
                .http
                .get(format!("{}/{LIST_SCOPE}", self.endpoints.base))
                .query(&query)
                .timeout(PROCESSING_STATUS_TIMEOUT)
                .send()
                .await
                .map_err(map_transport_err)?;
            let retry_after_seconds = parse_retry_after_seconds(response.headers());
            let body: ListResponse = response.json().await.map_err(|e| PublishError::Internal {
                detail: e.to_string(),
            })?;
            if body.base.result != 1 {
                return Err(classify_result(&body.base.error_msg, retry_after_seconds));
            }
            if let Some(video) = body.video_list.iter().find(|v| &v.photo_id == photo_id) {
                return Ok(if video.pending {
                    RemoteUploadState::RemoteProcessing
                } else {
                    RemoteUploadState::RemoteSucceeded
                });
            }
            if body.video_list.is_empty() {
                break;
            }
            cursor = body
                .video_list
                .last()
                .map(|v| v.photo_id.clone())
                .unwrap_or_default();
        }
        // Not found within the bounded scan — genuinely unknown, never
        // guessed as success or failure.
        Ok(RemoteUploadState::RemoteUnknown)
    }

    async fn recover_upload(
        &self,
        access_token: &str,
        mut session: UploadSession,
        video: &Video,
    ) -> Result<UploadSession, PublishError> {
        if session.state != RemoteUploadState::Transferring && session.state.safe_to_restart() {
            return Ok(session);
        }

        if session.remote_publish_id.is_some() {
            match self.get_remote_status(access_token, &session).await {
                Ok(RemoteUploadState::RemoteSucceeded) => {
                    session.state = RemoteUploadState::RemoteSucceeded;
                    return Ok(session);
                }
                Ok(RemoteUploadState::RemoteFailed) => {
                    session.state = RemoteUploadState::RemoteFailed;
                    return Ok(session);
                }
                _ => {
                    // Still processing, or genuinely unknown — either
                    // way, publish was already attempted, so this must
                    // not be retried automatically (section 69).
                    session.state = RemoteUploadState::RemoteUnknown;
                    return Ok(session);
                }
            }
        }

        let (Some(upload_host), Some(upload_token)) = (
            session.remote_upload_url.clone(),
            session.remote_session_id.clone(),
        ) else {
            session.state = RemoteUploadState::NotStarted;
            return Ok(session);
        };

        // Section 42/43: query the transfer's actual committed fragment
        // range before touching it again, exactly like YouTube's
        // Content-Range probe and TikTok's status poll.
        let response = self
            .http
            .get(format!("{upload_host}/{RESUME_SCOPE}"))
            .query(&[("upload_token", upload_token.as_str())])
            .timeout(UPLOAD_CONTROL_TIMEOUT)
            .send()
            .await
            .map_err(map_transport_err)?;
        let resume: ResumeResponse = response.json().await.map_err(|e| PublishError::Internal {
            detail: e.to_string(),
        })?;
        if resume.base.result != 1 {
            session.state = RemoteUploadState::NotStarted;
            session.remote_upload_url = None;
            session.remote_session_id = None;
            return Ok(session);
        }
        session.bytes_committed = if resume.existed {
            resume.fragment_index_bytes
        } else {
            0
        };
        session.state = RemoteUploadState::Transferring;

        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();
        let (session, result) = self
            .upload_media(access_token, session, video, sender, CancelSignal::new())
            .await;
        result.map(|()| session)
    }

    async fn cancel_upload_if_supported(
        &self,
        _access_token: &str,
        _session: &UploadSession,
    ) -> Result<(), PublishError> {
        // No cancel/delete-in-progress-upload endpoint was found in the
        // available (unofficial) API surface.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::domain::platform::Platform;
    use crate::domain::platform_account::PlatformAccount;
    use crate::test_support::{temp_dir, write_fake_video};

    fn sample_metadata() -> RenderedMetadata {
        RenderedMetadata {
            title: "A great clip".to_string(),
            description: String::new(),
            hashtags: vec![],
            provider_options: serde_json::json!({}),
        }
    }

    fn sample_video(path: &std::path::Path, size: i64) -> Video {
        Video::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            None,
            "clip.mp4",
            "Clip",
            path.display().to_string(),
            size,
            "mp4",
        )
    }

    #[tokio::test]
    async fn initialize_upload_captures_the_upload_token_and_host() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!("/{START_UPLOAD_SCOPE}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": 1,
                "error_msg": "",
                "upload_token": "tok-123",
                "endpoint": "upload.example.com"
            })))
            .mount(&server)
            .await;

        let uploader = KwaiUploader::with_base_url(&server.uri(), "app-1");
        let account = PlatformAccount::new(Uuid::new_v4(), Uuid::new_v4(), Platform::Kwai);
        let dir = temp_dir("kwai-uploader-init");
        let file_path = write_fake_video(&dir, "clip.mp4", b"0123456789");
        let video = sample_video(&file_path, 10);

        let session = uploader
            .initialize_upload(&account, "access-token", &video, &sample_metadata())
            .await
            .unwrap();

        assert_eq!(session.remote_session_id.as_deref(), Some("tok-123"));
        assert_eq!(
            session.remote_upload_url.as_deref(),
            Some("https://upload.example.com")
        );
        assert_eq!(session.state, RemoteUploadState::Initialized);
    }

    #[tokio::test]
    async fn a_small_file_uploads_as_a_single_fragment() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!("/{FRAGMENT_SCOPE}")))
            .and(query_param("fragment_id", "0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": 1,
                "error_msg": ""
            })))
            .mount(&server)
            .await;

        let uploader = KwaiUploader::with_base_url(&server.uri(), "app-1");
        let dir = temp_dir("kwai-uploader-single-fragment");
        let file_path = write_fake_video(&dir, "clip.mp4", b"0123456789");
        let video = sample_video(&file_path, 10);

        let mut session = UploadSession::new(
            Uuid::nil(),
            Uuid::nil(),
            Platform::Kwai,
            SessionType::Stepwise,
        );
        session.remote_session_id = Some("tok-123".to_string());
        session.remote_upload_url = Some(server.uri());
        session.bytes_total = Some(10);
        session.state = RemoteUploadState::Initialized;

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let (session, result) = uploader
            .upload_media("access-token", session, &video, tx, CancelSignal::new())
            .await;

        assert!(result.is_ok(), "{result:?}");
        assert_eq!(session.state, RemoteUploadState::Transferred);
        assert_eq!(session.bytes_committed, 10);
    }

    #[tokio::test]
    async fn finalize_publication_captures_the_photo_id_and_pending_state() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(format!("/{COMPLETE_SCOPE}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": 1,
                "error_msg": ""
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path(format!("/{PUBLISH_SCOPE}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": 1,
                "error_msg": "",
                "video_info": { "photo_id": "photo-999", "pending": true }
            })))
            .mount(&server)
            .await;

        let uploader = KwaiUploader::with_base_url(&server.uri(), "app-1");
        let mut session = UploadSession::new(
            Uuid::nil(),
            Uuid::nil(),
            Platform::Kwai,
            SessionType::Stepwise,
        );
        session.remote_session_id = Some("tok-123".to_string());
        session.remote_upload_url = Some(server.uri());
        session.bytes_total = Some(10);
        session.state = RemoteUploadState::Transferred;

        let finalized = uploader
            .finalize_publication("access-token", session)
            .await
            .unwrap();

        assert_eq!(finalized.remote_publish_id.as_deref(), Some("photo-999"));
        assert_eq!(finalized.state, RemoteUploadState::RemoteProcessing);
    }

    #[tokio::test]
    async fn get_remote_status_finds_the_matching_photo_in_the_list() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!("/{LIST_SCOPE}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": 1,
                "error_msg": "",
                "video_list": [
                    { "photo_id": "other-1", "pending": false },
                    { "photo_id": "photo-999", "pending": false }
                ]
            })))
            .mount(&server)
            .await;

        let uploader = KwaiUploader::with_base_url(&server.uri(), "app-1");
        let mut session = UploadSession::new(
            Uuid::nil(),
            Uuid::nil(),
            Platform::Kwai,
            SessionType::Stepwise,
        );
        session.remote_publish_id = Some("photo-999".to_string());

        let state = uploader
            .get_remote_status("access-token", &session)
            .await
            .unwrap();
        assert_eq!(state, RemoteUploadState::RemoteSucceeded);
    }

    #[tokio::test]
    async fn get_remote_status_is_unknown_when_the_photo_is_not_found() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(format!("/{LIST_SCOPE}")))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": 1,
                "error_msg": "",
                "video_list": []
            })))
            .mount(&server)
            .await;

        let uploader = KwaiUploader::with_base_url(&server.uri(), "app-1");
        let mut session = UploadSession::new(
            Uuid::nil(),
            Uuid::nil(),
            Platform::Kwai,
            SessionType::Stepwise,
        );
        session.remote_publish_id = Some("photo-999".to_string());

        let state = uploader
            .get_remote_status("access-token", &session)
            .await
            .unwrap();
        assert_eq!(state, RemoteUploadState::RemoteUnknown);
    }

    #[test]
    fn validate_metadata_rejects_an_empty_caption() {
        let uploader = KwaiUploader::with_base_url("https://example.com", "app-1");
        let mut metadata = sample_metadata();
        metadata.title = "   ".to_string();
        assert!(uploader.validate_metadata(&metadata).is_err());
    }

    #[test]
    fn fragment_count_rounds_up() {
        assert_eq!(fragment_count(1), 1);
        assert_eq!(fragment_count(FRAGMENT_SIZE), 1);
        assert_eq!(fragment_count(FRAGMENT_SIZE + 1), 2);
    }
}
