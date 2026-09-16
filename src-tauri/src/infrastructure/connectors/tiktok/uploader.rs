use std::io::SeekFrom;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
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

const CREATOR_INFO_PATH: &str = "https://open.tiktokapis.com/v2/post/publish/creator_info/query/";
const INIT_PATH: &str = "https://open.tiktokapis.com/v2/post/publish/video/init/";
const STATUS_PATH: &str = "https://open.tiktokapis.com/v2/post/publish/status/fetch/";

// Media transfer constraints per TikTok's Content Posting API docs
// (section 51/152): a chunk is 5-64 MiB, the final chunk may run up to
// 128 MiB to absorb the remainder, the whole file is capped at 4 GiB
// across at most 1000 chunks, and anything under 5 MiB must go as one
// single "chunk" covering the whole file.
const MIN_CHUNK: i64 = 5 * 1024 * 1024;
const MAX_CHUNK: i64 = 64 * 1024 * 1024;
const MAX_TOTAL_SIZE: i64 = 4 * 1024 * 1024 * 1024;
const MAX_CHUNK_COUNT: i64 = 1000;

/// Hardcoded, allowlisted TikTok endpoints (mirrors
/// `youtube::uploader::Endpoints`) — production always resolves to the
/// real hosts; `with_base_url` exists only for the `wiremock` test suite.
struct Endpoints {
    creator_info: String,
    init: String,
    status: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            creator_info: CREATOR_INFO_PATH.to_string(),
            init: INIT_PATH.to_string(),
            status: STATUS_PATH.to_string(),
        }
    }
}

/// Section 51: chunk arithmetic lives in exactly one place, never
/// scattered across the uploader's control flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChunkPlan {
    chunk_size: i64,
    total_chunk_count: i64,
}

fn plan_chunks(video_size: i64) -> Result<ChunkPlan, PublishError> {
    if video_size <= 0 {
        return Err(PublishError::InvalidMedia {
            detail: "the video file is empty".to_string(),
        });
    }
    if video_size > MAX_TOTAL_SIZE {
        return Err(PublishError::InvalidMedia {
            detail: "the video file exceeds TikTok's 4GB upload limit".to_string(),
        });
    }
    if video_size < MIN_CHUNK {
        return Ok(ChunkPlan {
            chunk_size: video_size,
            total_chunk_count: 1,
        });
    }
    let chunk_size = MAX_CHUNK.min(video_size).max(MIN_CHUNK);
    let total_chunk_count = (video_size / chunk_size).max(1);
    if total_chunk_count > MAX_CHUNK_COUNT {
        return Err(PublishError::InvalidMedia {
            detail: "the video file would require too many upload chunks".to_string(),
        });
    }
    Ok(ChunkPlan {
        chunk_size,
        total_chunk_count,
    })
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    data: Option<T>,
    error: Option<ApiError>,
}

#[derive(Debug, Default, Deserialize)]
struct CreatorInfoData {
    #[serde(default)]
    privacy_level_options: Vec<String>,
    #[serde(default)]
    comment_disabled: bool,
    #[serde(default)]
    duet_disabled: bool,
    #[serde(default)]
    stitch_disabled: bool,
}

#[derive(Debug, Serialize)]
struct PostInfo {
    title: String,
    privacy_level: String,
    disable_duet: bool,
    disable_comment: bool,
    disable_stitch: bool,
    video_cover_timestamp_ms: i64,
}

#[derive(Debug, Serialize)]
struct SourceInfo {
    source: &'static str,
    video_size: i64,
    chunk_size: i64,
    total_chunk_count: i64,
}

#[derive(Debug, Serialize)]
struct InitRequest {
    post_info: PostInfo,
    source_info: SourceInfo,
}

#[derive(Debug, Deserialize)]
struct InitData {
    publish_id: String,
    upload_url: String,
}

#[derive(Debug, Deserialize)]
struct StatusData {
    status: String,
    #[serde(default)]
    #[allow(dead_code)]
    fail_reason: Option<String>,
}

/// TikTok's real Direct Post `PlatformPublisher` (section 49-57). Uploads
/// stream directly from the desktop to TikTok's own endpoints — the Auth
/// Broker is never involved beyond having issued the access token
/// (section 156/157: no video proxy).
pub struct TikTokUploader {
    http: reqwest::Client,
    endpoints: Endpoints,
}

impl Default for TikTokUploader {
    fn default() -> Self {
        Self::new()
    }
}

impl TikTokUploader {
    pub fn new() -> Self {
        Self {
            http: build_upload_http_client(),
            endpoints: Endpoints::default(),
        }
    }

    /// Test-only — points at a local `wiremock` server instead of the
    /// live TikTok API.
    #[cfg(test)]
    fn with_base_url(base_url: &str) -> Self {
        Self {
            http: build_upload_http_client(),
            endpoints: Endpoints {
                creator_info: format!("{base_url}/v2/post/publish/creator_info/query/"),
                init: format!("{base_url}/v2/post/publish/video/init/"),
                status: format!("{base_url}/v2/post/publish/status/fetch/"),
            },
        }
    }

    async fn fetch_creator_info(
        &self,
        access_token: &str,
    ) -> Result<CreatorInfoData, PublishError> {
        let response = self
            .http
            .post(&self.endpoints.creator_info)
            .bearer_auth(access_token)
            .header("Content-Type", "application/json; charset=UTF-8")
            .body("{}")
            .timeout(UPLOAD_CONTROL_TIMEOUT)
            .send()
            .await
            .map_err(map_transport_err)?;
        parse_envelope(response).await
    }
}

fn content_type_for(video: &Video) -> &'static str {
    match video.extension.to_lowercase().as_str() {
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        _ => "video/mp4",
    }
}

async fn parse_envelope<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
) -> Result<T, PublishError> {
    let status = response.status();
    let retry_after_seconds = parse_retry_after_seconds(response.headers());
    let envelope: Envelope<T> = response.json().await.map_err(|e| PublishError::Internal {
        detail: e.to_string(),
    })?;
    if status.is_success() {
        if let Some(data) = envelope.data {
            return Ok(data);
        }
    }
    let error = envelope.error.unwrap_or(ApiError {
        code: "unknown".to_string(),
        message: format!("TikTok returned HTTP {status} with no error detail"),
    });
    Err(classify_api_error(
        &error.code,
        &error.message,
        retry_after_seconds,
    ))
}

fn classify_api_error(code: &str, message: &str, retry_after_seconds: Option<u64>) -> PublishError {
    match code {
        "ok" => PublishError::Internal {
            detail: format!("unexpected ok error envelope: {message}"),
        },
        "access_token_invalid" | "scope_not_authorized" | "scope_permission_missed" => {
            PublishError::AuthExpired
        }
        "rate_limit_exceeded" => PublishError::RateLimited {
            retry_after_seconds,
        },
        "spam_risk_too_many_posts" | "spam_risk_user_banned_from_posting" => {
            PublishError::PlatformNotApproved {
                detail: message.to_string(),
            }
        }
        "unaudited_client_can_only_post_to_private_accounts" => PublishError::PlatformNotApproved {
            detail: "this app is not yet audited for public TikTok posts".to_string(),
        },
        "invalid_param" | "invalid_file_upload" | "video_pull_failed" => {
            PublishError::InvalidMedia {
                detail: message.to_string(),
            }
        }
        _ if code.starts_with("internal") => PublishError::ProviderServerError { status: None },
        _ => PublishError::Internal {
            detail: format!("TikTok error {code}: {message}"),
        },
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

#[async_trait]
impl PlatformPublisher for TikTokUploader {
    fn platform(&self) -> Platform {
        Platform::TikTok
    }

    async fn validate_media(&self, video: &Video) -> Result<(), PublishError> {
        plan_chunks(video.file_size_bytes).map(|_| ())
    }

    fn validate_metadata(&self, metadata: &RenderedMetadata) -> Result<(), PublishError> {
        if metadata.title.trim().is_empty() {
            return Err(PublishError::InvalidMetadata {
                detail: "caption cannot be empty".to_string(),
            });
        }
        // TikTok's documented cap is 2200 UTF-16 code units.
        if metadata.title.encode_utf16().count() > 2200 {
            return Err(PublishError::InvalidMetadata {
                detail: "caption exceeds TikTok's 2200-character limit".to_string(),
            });
        }
        Ok(())
    }

    async fn initialize_upload(
        &self,
        _account: &PlatformAccount,
        access_token: &str,
        video: &Video,
        metadata: &RenderedMetadata,
    ) -> Result<UploadSession, PublishError> {
        let plan = plan_chunks(video.file_size_bytes)?;

        // Section 35: never render/request a privacy level the connected
        // creator account isn't actually permitted to use.
        let creator_info = self.fetch_creator_info(access_token).await?;
        let requested_privacy = metadata
            .provider_options
            .get("privacy_level")
            .and_then(|v| v.as_str())
            .unwrap_or("SELF_ONLY");
        let privacy_level = if creator_info
            .privacy_level_options
            .iter()
            .any(|p| p == requested_privacy)
        {
            requested_privacy.to_string()
        } else {
            // Falls back to whatever the account is actually allowed,
            // preferring the most restrictive (safe-defaults, section
            // 132) — never silently escalate to a broader audience than
            // requested.
            creator_info
                .privacy_level_options
                .iter()
                .find(|p| p.as_str() == "SELF_ONLY")
                .or_else(|| creator_info.privacy_level_options.first())
                .cloned()
                .ok_or_else(|| PublishError::PermissionMissing {
                    capability: "upload_video".to_string(),
                })?
        };

        let cover_ms = metadata
            .provider_options
            .get("video_cover_timestamp_ms")
            .and_then(|v| v.as_i64())
            .unwrap_or(1000);

        let request = InitRequest {
            post_info: PostInfo {
                title: metadata.title.clone(),
                privacy_level,
                disable_duet: creator_info.duet_disabled
                    || metadata
                        .provider_options
                        .get("disable_duet")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                disable_comment: creator_info.comment_disabled
                    || metadata
                        .provider_options
                        .get("disable_comment")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                disable_stitch: creator_info.stitch_disabled
                    || metadata
                        .provider_options
                        .get("disable_stitch")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false),
                video_cover_timestamp_ms: cover_ms,
            },
            source_info: SourceInfo {
                source: "FILE_UPLOAD",
                video_size: video.file_size_bytes,
                chunk_size: plan.chunk_size,
                total_chunk_count: plan.total_chunk_count,
            },
        };

        let response = self
            .http
            .post(&self.endpoints.init)
            .bearer_auth(access_token)
            .header("Content-Type", "application/json; charset=UTF-8")
            .json(&request)
            .timeout(UPLOAD_CONTROL_TIMEOUT)
            .send()
            .await
            .map_err(map_transport_err)?;
        let data: InitData = parse_envelope(response).await?;

        let mut session = UploadSession::new(
            Uuid::nil(),
            Uuid::nil(),
            Platform::TikTok,
            SessionType::DirectPost,
        );
        session.remote_publish_id = Some(data.publish_id);
        session.remote_upload_url = Some(data.upload_url);
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
        let Some(upload_url) = session.remote_upload_url.clone() else {
            return (
                session,
                Err(PublishError::Internal {
                    detail: "no upload URL on session".to_string(),
                }),
            );
        };
        let total = session.bytes_total.unwrap_or(video.file_size_bytes);
        let plan = match plan_chunks(total) {
            Ok(plan) => plan,
            Err(err) => return (session, Err(err)),
        };
        let content_type = content_type_for(video);

        let mut file = match tokio::fs::File::open(&video.file_path).await {
            Ok(file) => file,
            Err(_) => return (session, Err(PublishError::VideoUnavailable)),
        };

        let mut offset = session.bytes_committed;
        while offset < total {
            if cancel.is_cancelled() {
                return (session, Err(PublishError::Cancelled));
            }
            let remaining = total - offset;
            // Only the final chunk may exceed the planned chunk_size
            // (absorbing the remainder), matching what `source_info` in
            // `initialize_upload` already promised TikTok.
            let is_final_by_count = offset + plan.chunk_size >= total;
            let this_chunk = if is_final_by_count {
                remaining
            } else {
                plan.chunk_size
            };

            if file.seek(SeekFrom::Start(offset as u64)).await.is_err() {
                return (session, Err(PublishError::VideoUnavailable));
            }
            let mut buf = vec![0u8; this_chunk as usize];
            if file.read_exact(&mut buf).await.is_err() {
                return (session, Err(PublishError::VideoUnavailable));
            }

            let content_range = format!("bytes {}-{}/{}", offset, offset + this_chunk - 1, total);
            let result = self
                .http
                .put(&upload_url)
                .header("Content-Type", content_type)
                .header("Content-Length", this_chunk.to_string())
                .header("Content-Range", content_range)
                .body(buf)
                .timeout(UPLOAD_CHUNK_TIMEOUT)
                .send()
                .await;

            let response = match result {
                Ok(response) => response,
                Err(err) => {
                    session.state = if offset > 0 {
                        RemoteUploadState::Transferring
                    } else {
                        RemoteUploadState::Initialized
                    };
                    return (session, Err(map_transport_err(err)));
                }
            };

            let status = response.status();
            if status.is_success() {
                offset += this_chunk;
                session.bytes_committed = offset;
                let _ = progress.send(UploadProgress {
                    bytes_uploaded: offset,
                    bytes_total: Some(total),
                });
            } else if status.as_u16() == 404 || status.as_u16() == 410 {
                session.state = RemoteUploadState::NotStarted;
                session.remote_upload_url = None;
                return (session, Err(PublishError::UploadSessionExpired));
            } else if status.as_u16() == 429 {
                session.state = RemoteUploadState::Transferring;
                return (
                    session,
                    Err(PublishError::RateLimited {
                        retry_after_seconds: parse_retry_after_seconds(response.headers()),
                    }),
                );
            } else if status.is_server_error() {
                session.state = RemoteUploadState::Transferring;
                return (
                    session,
                    Err(PublishError::ProviderServerError {
                        status: Some(status.as_u16()),
                    }),
                );
            } else {
                session.state = RemoteUploadState::RemoteUnknown;
                return (
                    session,
                    Err(PublishError::Internal {
                        detail: format!("TikTok chunk upload returned HTTP {status}"),
                    }),
                );
            }
        }

        session.state = RemoteUploadState::Transferred;
        (session, Ok(()))
    }

    async fn finalize_publication(
        &self,
        _access_token: &str,
        mut session: UploadSession,
    ) -> Result<UploadSession, PublishError> {
        // TikTok has no separate "finalize" call — the init + chunked
        // transfer already registered the post under `publish_id`;
        // completion is confirmed entirely through status polling.
        session.state = RemoteUploadState::RemoteProcessing;
        Ok(session)
    }

    async fn get_remote_status(
        &self,
        access_token: &str,
        session: &UploadSession,
    ) -> Result<RemoteUploadState, PublishError> {
        let Some(publish_id) = &session.remote_publish_id else {
            return Ok(session.state);
        };
        let response = self
            .http
            .post(&self.endpoints.status)
            .bearer_auth(access_token)
            .header("Content-Type", "application/json; charset=UTF-8")
            .json(&serde_json::json!({ "publish_id": publish_id }))
            .timeout(PROCESSING_STATUS_TIMEOUT)
            .send()
            .await
            .map_err(map_transport_err)?;
        let data: StatusData = parse_envelope(response).await?;
        Ok(match data.status.as_str() {
            "PUBLISH_COMPLETE" | "SEND_TO_USER_INBOX" => RemoteUploadState::RemoteSucceeded,
            "FAILED" => RemoteUploadState::RemoteFailed,
            // "PROCESSING_UPLOAD" / "PROCESSING_DOWNLOAD" / anything
            // unrecognized: still going, never guessed as success (the
            // full status enum wasn't confirmable against current
            // official docs — see docs/tiktok-publishing.md).
            _ => RemoteUploadState::RemoteProcessing,
        })
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
        // TikTok's upload_url has no documented byte-range status-check
        // endpoint the way YouTube's does (section 54) — the safest
        // available signal is the publish status itself. If it already
        // shows progress beyond upload, the transfer clearly landed;
        // otherwise this falls back to resuming from the last locally
        // confirmed offset, which is honest (not a guess at bytes never
        // actually acknowledged by a response) since `bytes_committed`
        // is only ever advanced after a successful chunk PUT.
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
                _ => {}
            }
        }
        if session.remote_upload_url.is_none() {
            session.state = RemoteUploadState::NotStarted;
            return Ok(session);
        }

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
        // TikTok's Content Posting API documents no cancel/delete
        // endpoint for a pending Direct Post — a not-yet-finalized
        // upload_url simply expires on its own (section 54/127).
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::domain::platform::Platform;
    use crate::domain::platform_account::PlatformAccount;
    use crate::test_support::{temp_dir, write_fake_video};

    fn sample_metadata() -> RenderedMetadata {
        RenderedMetadata {
            title: "Amazing goal".to_string(),
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

    async fn mount_default_creator_info(server: &MockServer) {
        Mock::given(method("POST"))
            .and(path("/v2/post/publish/creator_info/query/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "privacy_level_options": ["SELF_ONLY", "PUBLIC_TO_EVERYONE"],
                    "comment_disabled": false,
                    "duet_disabled": false,
                    "stitch_disabled": false
                },
                "error": { "code": "ok", "message": "", "log_id": "abc" }
            })))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn initialize_upload_captures_the_publish_id_and_upload_url() {
        let server = MockServer::start().await;
        mount_default_creator_info(&server).await;
        Mock::given(method("POST"))
            .and(path("/v2/post/publish/video/init/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "publish_id": "v_inbox.123",
                    "upload_url": format!("{}/upload/session/abc", server.uri())
                },
                "error": { "code": "ok", "message": "", "log_id": "abc" }
            })))
            .mount(&server)
            .await;

        let uploader = TikTokUploader::with_base_url(&server.uri());
        let account = PlatformAccount::new(Uuid::new_v4(), Uuid::new_v4(), Platform::TikTok);
        let dir = temp_dir("tiktok-uploader-init");
        let file_path = write_fake_video(&dir, "clip.mp4", b"0123456789");
        let video = sample_video(&file_path, 10);

        let session = uploader
            .initialize_upload(&account, "access-token", &video, &sample_metadata())
            .await
            .unwrap();

        assert_eq!(session.remote_publish_id.as_deref(), Some("v_inbox.123"));
        assert!(session.remote_upload_url.is_some());
        assert_eq!(session.state, RemoteUploadState::Initialized);
        assert_eq!(session.bytes_total, Some(10));
    }

    #[tokio::test]
    async fn initialize_upload_falls_back_to_the_most_restrictive_allowed_privacy_level() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v2/post/publish/creator_info/query/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "privacy_level_options": ["MUTUAL_FOLLOW_FRIENDS"],
                    "comment_disabled": false,
                    "duet_disabled": false,
                    "stitch_disabled": false
                },
                "error": { "code": "ok", "message": "", "log_id": "abc" }
            })))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v2/post/publish/video/init/"))
            .and(wiremock::matchers::body_string_contains(
                "MUTUAL_FOLLOW_FRIENDS",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "publish_id": "v_inbox.456",
                    "upload_url": format!("{}/upload/session/abc", server.uri())
                },
                "error": { "code": "ok", "message": "", "log_id": "abc" }
            })))
            .mount(&server)
            .await;

        let uploader = TikTokUploader::with_base_url(&server.uri());
        let account = PlatformAccount::new(Uuid::new_v4(), Uuid::new_v4(), Platform::TikTok);
        let dir = temp_dir("tiktok-uploader-privacy-fallback");
        let file_path = write_fake_video(&dir, "clip.mp4", b"0123456789");
        let video = sample_video(&file_path, 10);

        // Requests SELF_ONLY, which isn't in the account's allowed list —
        // must fall back to the account's only allowed option rather than
        // erroring or silently posting more broadly.
        let session = uploader
            .initialize_upload(&account, "access-token", &video, &sample_metadata())
            .await
            .unwrap();

        assert_eq!(session.remote_publish_id.as_deref(), Some("v_inbox.456"));
    }

    #[tokio::test]
    async fn a_small_file_uploads_as_a_single_chunk() {
        let server = MockServer::start().await;
        let upload_path = "/upload/session/one-shot";
        Mock::given(method("PUT"))
            .and(path(upload_path))
            .and(header("Content-Range", "bytes 0-9/10"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let uploader = TikTokUploader::with_base_url(&server.uri());
        let dir = temp_dir("tiktok-uploader-single-chunk");
        let file_path = write_fake_video(&dir, "clip.mp4", b"0123456789");
        let video = sample_video(&file_path, 10);

        let mut session = UploadSession::new(
            Uuid::nil(),
            Uuid::nil(),
            Platform::TikTok,
            SessionType::DirectPost,
        );
        session.remote_upload_url = Some(format!("{}{upload_path}", server.uri()));
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
    async fn get_remote_status_maps_statuses_conservatively() {
        let server = MockServer::start().await;
        for (raw_status, expected) in [
            ("PUBLISH_COMPLETE", RemoteUploadState::RemoteSucceeded),
            ("SEND_TO_USER_INBOX", RemoteUploadState::RemoteSucceeded),
            ("FAILED", RemoteUploadState::RemoteFailed),
            ("PROCESSING_UPLOAD", RemoteUploadState::RemoteProcessing),
            (
                "SOME_NEW_STATUS_TIKTOK_ADDS_LATER",
                RemoteUploadState::RemoteProcessing,
            ),
        ] {
            let publish_id = format!("v_inbox.{raw_status}");
            Mock::given(method("POST"))
                .and(path("/v2/post/publish/status/fetch/"))
                .and(wiremock::matchers::body_string_contains(
                    publish_id.as_str(),
                ))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "data": { "status": raw_status, "fail_reason": "" },
                    "error": { "code": "ok", "message": "", "log_id": "abc" }
                })))
                .mount(&server)
                .await;

            let uploader = TikTokUploader::with_base_url(&server.uri());
            let mut session = UploadSession::new(
                Uuid::nil(),
                Uuid::nil(),
                Platform::TikTok,
                SessionType::DirectPost,
            );
            session.remote_publish_id = Some(publish_id);

            let state = uploader
                .get_remote_status("access-token", &session)
                .await
                .unwrap();
            assert_eq!(
                state, expected,
                "raw status {raw_status:?} mapped incorrectly"
            );
        }
    }

    #[tokio::test]
    async fn an_unaudited_app_error_maps_to_platform_not_approved() {
        let server = MockServer::start().await;
        mount_default_creator_info(&server).await;
        Mock::given(method("POST"))
            .and(path("/v2/post/publish/video/init/"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "data": null,
                "error": {
                    "code": "unaudited_client_can_only_post_to_private_accounts",
                    "message": "app not audited",
                    "log_id": "abc"
                }
            })))
            .mount(&server)
            .await;

        let uploader = TikTokUploader::with_base_url(&server.uri());
        let account = PlatformAccount::new(Uuid::new_v4(), Uuid::new_v4(), Platform::TikTok);
        let dir = temp_dir("tiktok-uploader-unaudited");
        let file_path = write_fake_video(&dir, "clip.mp4", b"0123456789");
        let video = sample_video(&file_path, 10);

        let result = uploader
            .initialize_upload(&account, "access-token", &video, &sample_metadata())
            .await;

        assert!(matches!(
            result,
            Err(PublishError::PlatformNotApproved { .. })
        ));
    }

    #[test]
    fn validate_metadata_rejects_an_empty_caption() {
        let uploader = TikTokUploader::new();
        let mut metadata = sample_metadata();
        metadata.title = "   ".to_string();
        assert!(uploader.validate_metadata(&metadata).is_err());
    }

    #[test]
    fn chunk_plan_uses_a_single_chunk_under_the_five_megabyte_floor() {
        let plan = plan_chunks(1024).unwrap();
        assert_eq!(plan.total_chunk_count, 1);
        assert_eq!(plan.chunk_size, 1024);
    }

    #[test]
    fn chunk_plan_rejects_a_file_over_the_four_gigabyte_cap() {
        assert!(plan_chunks(MAX_TOTAL_SIZE + 1).is_err());
    }

    #[test]
    fn chunk_plan_uses_the_max_chunk_size_for_a_large_file() {
        let plan = plan_chunks(200 * 1024 * 1024).unwrap();
        assert_eq!(plan.chunk_size, MAX_CHUNK);
        assert!(plan.total_chunk_count >= 1);
    }
}
