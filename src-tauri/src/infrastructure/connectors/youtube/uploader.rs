use std::io::SeekFrom;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
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

const UPLOAD_ENDPOINT: &str = "https://www.googleapis.com/upload/youtube/v3/videos";
const VIDEOS_ENDPOINT: &str = "https://www.googleapis.com/youtube/v3/videos";

/// Hardcoded, allowlisted Google endpoints (section 25's no-generic-proxy
/// principle) — production code always uses `YouTubeUploader::new()`,
/// which resolves to the real hosts above. `with_base_url` exists only so
/// the test suite can point this at a local `wiremock` server instead of
/// a live Google endpoint; it is not reachable from any production code
/// path.
struct Endpoints {
    upload: String,
    videos: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            upload: UPLOAD_ENDPOINT.to_string(),
            videos: VIDEOS_ENDPOINT.to_string(),
        }
    }
}

/// Every chunk but the last must be a multiple of 256 KiB per Google's
/// resumable upload protocol (verified against the official docs —
/// section 152). 8 MiB balances progress granularity against per-chunk
/// HTTP overhead; still small enough that only one chunk is ever held in
/// memory at once (section 41/116).
const CHUNK_SIZE: i64 = 8 * 1024 * 1024;

#[derive(Debug, Deserialize)]
struct VideoResource {
    id: String,
    #[serde(rename = "processingDetails")]
    processing_details: Option<ProcessingDetails>,
}

#[derive(Debug, Deserialize)]
struct ProcessingDetails {
    #[serde(rename = "processingStatus")]
    processing_status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VideoListResponse {
    items: Vec<VideoResource>,
}

/// YouTube's real resumable-upload `PlatformPublisher` (section 40-48).
/// Talks directly to Google — no broker involvement, matching every
/// other YouTube operation in this codebase.
pub struct YouTubeUploader {
    http: reqwest::Client,
    endpoints: Endpoints,
    chunk_size: i64,
}

impl Default for YouTubeUploader {
    fn default() -> Self {
        Self::new()
    }
}

impl YouTubeUploader {
    pub fn new() -> Self {
        Self {
            http: build_upload_http_client(),
            endpoints: Endpoints::default(),
            chunk_size: CHUNK_SIZE,
        }
    }

    /// Test-only — points at a local `wiremock` server and (since real
    /// multi-chunk coverage would otherwise need an 8 MiB+ fixture file
    /// on every test run) shrinks the chunk size so the real 308-based
    /// continuation logic is exercised against a small fixture instead.
    #[cfg(test)]
    fn with_base_url(base_url: &str, chunk_size: i64) -> Self {
        Self {
            http: build_upload_http_client(),
            endpoints: Endpoints {
                upload: format!("{base_url}/upload/youtube/v3/videos"),
                videos: format!("{base_url}/youtube/v3/videos"),
            },
            chunk_size,
        }
    }
}

#[async_trait]
impl PlatformPublisher for YouTubeUploader {
    fn platform(&self) -> Platform {
        Platform::YouTube
    }

    async fn validate_media(&self, video: &Video) -> Result<(), PublishError> {
        if video.file_size_bytes <= 0 {
            return Err(PublishError::InvalidMedia {
                detail: "the video file is empty".to_string(),
            });
        }
        // YouTube's actual cap depends on account verification status
        // (256GB/12h verified, 15GB/15min otherwise) — this is a
        // deliberately generous sanity bound, not a claim of the full
        // picture; the provider's own rejection is authoritative.
        const GENEROUS_MAX_BYTES: i64 = 256 * 1024 * 1024 * 1024;
        if video.file_size_bytes > GENEROUS_MAX_BYTES {
            return Err(PublishError::InvalidMedia {
                detail: "the video file exceeds YouTube's maximum upload size".to_string(),
            });
        }
        Ok(())
    }

    fn validate_metadata(&self, metadata: &RenderedMetadata) -> Result<(), PublishError> {
        if metadata.title.trim().is_empty() {
            return Err(PublishError::InvalidMetadata {
                detail: "title cannot be empty".to_string(),
            });
        }
        if metadata.title.chars().count() > 100 {
            return Err(PublishError::InvalidMetadata {
                detail: "title exceeds YouTube's 100-character limit".to_string(),
            });
        }
        if metadata.description.chars().count() > 5000 {
            return Err(PublishError::InvalidMetadata {
                detail: "description exceeds YouTube's 5000-character limit".to_string(),
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
        // Defaults are deliberately conservative (private, generic
        // category) — never publish more broadly than the caller
        // explicitly configured (section 47/132's safe-defaults
        // philosophy) until the metadata template system supplies real
        // provider_options.
        let privacy_status = metadata
            .provider_options
            .get("privacy_status")
            .and_then(|v| v.as_str())
            .unwrap_or("private");
        let category_id = metadata
            .provider_options
            .get("category_id")
            .and_then(|v| v.as_str())
            .unwrap_or("22");
        let tags: Vec<String> = metadata
            .hashtags
            .iter()
            .map(|h| h.trim_start_matches('#').to_string())
            .collect();

        let body = json!({
            "snippet": {
                "title": metadata.title,
                "description": metadata.description,
                "tags": tags,
                "categoryId": category_id,
            },
            "status": {
                "privacyStatus": privacy_status,
                "selfDeclaredMadeForKids": false,
            }
        });

        let response = self
            .http
            .post(&self.endpoints.upload)
            .query(&[("uploadType", "resumable"), ("part", "snippet,status")])
            .bearer_auth(access_token)
            .header("X-Upload-Content-Length", video.file_size_bytes.to_string())
            .header("X-Upload-Content-Type", "video/*")
            .json(&body)
            .timeout(UPLOAD_CONTROL_TIMEOUT)
            .send()
            .await
            .map_err(map_transport_err)?;

        if !response.status().is_success() {
            return Err(classify_response(&response));
        }
        let upload_url = response
            .headers()
            .get("location")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| PublishError::Internal {
                detail: "YouTube did not return a resumable session Location header".to_string(),
            })?
            .to_string();

        let mut session = UploadSession::new(
            Uuid::nil(),
            Uuid::nil(),
            Platform::YouTube,
            SessionType::Resumable,
        );
        session.remote_upload_url = Some(upload_url);
        session.bytes_total = Some(video.file_size_bytes);
        session.state = RemoteUploadState::Initialized;
        Ok(session)
    }

    async fn upload_media(
        &self,
        access_token: &str,
        mut session: UploadSession,
        video: &Video,
        progress: ProgressSender,
        cancel: CancelSignal,
    ) -> (UploadSession, Result<(), PublishError>) {
        let Some(upload_url) = session.remote_upload_url.clone() else {
            return (
                session,
                Err(PublishError::Internal {
                    detail: "no resumable upload URL on session".to_string(),
                }),
            );
        };
        let total = session.bytes_total.unwrap_or(video.file_size_bytes);

        let mut file = match tokio::fs::File::open(&video.file_path).await {
            Ok(file) => file,
            Err(_) => return (session, Err(PublishError::VideoUnavailable)),
        };

        let mut offset = session.bytes_committed;
        loop {
            if cancel.is_cancelled() {
                return (session, Err(PublishError::Cancelled));
            }
            if offset >= total {
                break;
            }
            let this_chunk = (total - offset).min(self.chunk_size);
            let is_final = offset + this_chunk >= total;

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
                .bearer_auth(access_token)
                .header("Content-Length", this_chunk.to_string())
                .header("Content-Range", content_range)
                .body(buf)
                .timeout(UPLOAD_CHUNK_TIMEOUT)
                .send()
                .await;

            let response = match result {
                Ok(response) => response,
                Err(err) => {
                    // Nothing confirmed for *this* chunk — bytes_committed
                    // stays at the last acknowledged offset, and the
                    // session is left `Transferring` (not safe to
                    // restart) so recovery re-queries the provider's
                    // actual committed range instead of assuming.
                    session.state = RemoteUploadState::Transferring;
                    return (session, Err(map_transport_err(err)));
                }
            };

            let status = response.status().as_u16();
            if is_final && (status == 200 || status == 201) {
                let resource: VideoResource = match response.json().await {
                    Ok(resource) => resource,
                    Err(_) => {
                        session.state = RemoteUploadState::RemoteUnknown;
                        return (session, Err(PublishError::UnknownRemoteResult));
                    }
                };
                session.bytes_committed = total;
                session.state = RemoteUploadState::Transferred;
                session.remote_publish_id = Some(resource.id);
                let _ = progress.send(UploadProgress {
                    bytes_uploaded: total,
                    bytes_total: Some(total),
                });
                break;
            } else if status == 308 {
                // Resume Incomplete — the expected response for every
                // non-final chunk.
                offset += this_chunk;
                session.bytes_committed = offset;
                let _ = progress.send(UploadProgress {
                    bytes_uploaded: offset,
                    bytes_total: Some(total),
                });
            } else if (500..600).contains(&status) {
                session.state = RemoteUploadState::Transferring;
                return (
                    session,
                    Err(PublishError::ProviderServerError {
                        status: Some(status),
                    }),
                );
            } else if status == 404 {
                // An expired session does not prove the final chunk was
                // rejected; its response may have been lost locally.
                session.state = RemoteUploadState::RemoteUnknown;
                return (session, Err(PublishError::UnknownRemoteResult));
            } else {
                session.state = RemoteUploadState::RemoteUnknown;
                return (session, Err(classify_response(&response)));
            }
        }

        (session, Ok(()))
    }

    async fn finalize_publication(
        &self,
        _access_token: &str,
        session: UploadSession,
    ) -> Result<UploadSession, PublishError> {
        // The resumable upload's completion response (handled inside
        // `upload_media`, since that's the call that actually receives
        // it) already *is* the created video resource — nothing further
        // to send here.
        Ok(session)
    }

    async fn get_remote_status(
        &self,
        access_token: &str,
        session: &UploadSession,
    ) -> Result<RemoteUploadState, PublishError> {
        let Some(video_id) = &session.remote_publish_id else {
            return Ok(session.state);
        };
        let response = self
            .http
            .get(&self.endpoints.videos)
            .bearer_auth(access_token)
            .query(&[
                ("part", "status,processingDetails"),
                ("id", video_id.as_str()),
            ])
            .timeout(PROCESSING_STATUS_TIMEOUT)
            .send()
            .await
            .map_err(map_transport_err)?;
        if !response.status().is_success() {
            return Err(classify_response(&response));
        }
        let body: VideoListResponse =
            response.json().await.map_err(|e| PublishError::Internal {
                detail: e.to_string(),
            })?;
        let Some(item) = body.items.into_iter().next() else {
            return Ok(RemoteUploadState::RemoteUnknown);
        };
        let processing_status = item
            .processing_details
            .and_then(|d| d.processing_status)
            .unwrap_or_default();
        Ok(match processing_status.as_str() {
            "succeeded" => RemoteUploadState::RemoteSucceeded,
            "failed" | "terminated" => RemoteUploadState::RemoteFailed,
            // "processing" or an unrecognized value both mean "not done
            // yet, keep polling" — never guess success on an unknown
            // status string.
            _ => RemoteUploadState::RemoteProcessing,
        })
    }

    async fn reconcile_remote(
        &self,
        access_token: &str,
        session: &UploadSession,
    ) -> Result<UploadSession, PublishError> {
        let mut result = session.clone();
        if result.remote_publish_id.is_some() {
            result.state = self.get_remote_status(access_token, session).await?;
            return Ok(result);
        }
        let Some(upload_url) = &session.remote_upload_url else {
            result.state = RemoteUploadState::RemoteUnknown;
            return Ok(result);
        };
        let Some(total) = session.bytes_total else {
            result.state = RemoteUploadState::RemoteUnknown;
            return Ok(result);
        };
        // The zero-byte resumable status probe cannot create a video or
        // transfer media. An expired/404 session proves no such thing.
        let response = self
            .http
            .put(upload_url)
            .bearer_auth(access_token)
            .header("Content-Length", "0")
            .header("Content-Range", format!("bytes */{total}"))
            .timeout(UPLOAD_CONTROL_TIMEOUT)
            .send()
            .await
            .map_err(map_transport_err)?;
        match response.status().as_u16() {
            308 => {
                result.bytes_committed = response
                    .headers()
                    .get("range")
                    .and_then(|v| v.to_str().ok())
                    .and_then(parse_range_upper_bound)
                    .map(|upper| upper + 1)
                    .unwrap_or(0);
                result.state = RemoteUploadState::Transferring;
            }
            200 | 201 => {
                let resource = response
                    .json::<VideoResource>()
                    .await
                    .map_err(|_| PublishError::UnknownRemoteResult)?;
                result.remote_publish_id = Some(resource.id);
                result.bytes_committed = total;
                result.state = self.get_remote_status(access_token, &result).await?;
            }
            404 | 410 => result.state = RemoteUploadState::RemoteUnknown,
            _ => return Err(classify_response(&response)),
        }
        Ok(result)
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
        let Some(upload_url) = session.remote_upload_url.clone() else {
            session.state = RemoteUploadState::NotStarted;
            return Ok(session);
        };
        let total = session.bytes_total.unwrap_or(0);

        // Section 42/43: query the resumable session's actual committed
        // range before touching it again — the locally recorded
        // `bytes_committed` is never trusted blindly after a crash.
        let response = self
            .http
            .put(&upload_url)
            .bearer_auth(access_token)
            .header("Content-Length", "0")
            .header("Content-Range", format!("bytes */{total}"))
            .timeout(UPLOAD_CONTROL_TIMEOUT)
            .send()
            .await
            .map_err(map_transport_err)?;

        match response.status().as_u16() {
            308 => {
                let committed = response
                    .headers()
                    .get("range")
                    .and_then(|v| v.to_str().ok())
                    .and_then(parse_range_upper_bound)
                    .map(|upper| upper + 1)
                    .unwrap_or(0);
                session.bytes_committed = committed;
                session.state = RemoteUploadState::Transferring;
            }
            200 | 201 => {
                // The provider actually has it despite the local
                // interruption — never re-upload; recover the video id
                // from this same response instead.
                let resource = response
                    .json::<VideoResource>()
                    .await
                    .map_err(|_| PublishError::UnknownRemoteResult)?;
                session.remote_publish_id = Some(resource.id);
                session.bytes_committed = total;
                session.state = RemoteUploadState::Transferred;
                return Ok(session);
            }
            404 => {
                return Err(PublishError::UnknownRemoteResult);
            }
            _ => return Err(classify_response(&response)),
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
        session: &UploadSession,
    ) -> Result<(), PublishError> {
        // Best-effort only (section 127) — a not-yet-finalized resumable
        // session also expires on its own; this failing is never fatal.
        if let Some(url) = &session.remote_upload_url {
            let _ = self
                .http
                .delete(url)
                .timeout(UPLOAD_CONTROL_TIMEOUT)
                .send()
                .await;
        }
        Ok(())
    }
}

/// Parses the upper bound out of a `Range: bytes=0-1048575` response
/// header, per RFC 7233 — what Google's resumable upload status check
/// returns to report how many bytes it has actually received.
fn parse_range_upper_bound(range_header: &str) -> Option<i64> {
    range_header
        .strip_prefix("bytes=")?
        .split('-')
        .nth(1)?
        .parse()
        .ok()
}

fn classify_response(response: &reqwest::Response) -> PublishError {
    let status = response.status().as_u16();
    match status {
        401 | 403 => PublishError::AuthExpired,
        404 => PublishError::UploadSessionExpired,
        429 => PublishError::RateLimited {
            retry_after_seconds: parse_retry_after_seconds(response.headers()),
        },
        500..=599 => PublishError::ProviderServerError {
            status: Some(status),
        },
        _ => PublishError::Internal {
            detail: format!("YouTube returned HTTP {status}"),
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
            description: "Watch this incredible play.".to_string(),
            hashtags: vec!["#futebol".to_string(), "#shorts".to_string()],
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
    async fn initialize_upload_returns_a_resumable_session_from_the_location_header() {
        let server = MockServer::start().await;
        let location = format!("{}/upload/youtube/v3/videos?upload_id=abc123", server.uri());
        Mock::given(method("POST"))
            .and(path("/upload/youtube/v3/videos"))
            .respond_with(ResponseTemplate::new(200).insert_header("Location", location.as_str()))
            .mount(&server)
            .await;

        let uploader = YouTubeUploader::with_base_url(&server.uri(), 8 * 1024 * 1024);
        let account = PlatformAccount::new(Uuid::new_v4(), Uuid::new_v4(), Platform::YouTube);
        let dir = temp_dir("youtube-uploader-init");
        let file_path = write_fake_video(&dir, "clip.mp4", b"0123456789");
        let video = sample_video(&file_path, 10);

        let session = uploader
            .initialize_upload(&account, "access-token", &video, &sample_metadata())
            .await
            .unwrap();

        assert_eq!(
            session.remote_upload_url.as_deref(),
            Some(location.as_str())
        );
        assert_eq!(session.state, RemoteUploadState::Initialized);
        assert_eq!(session.bytes_total, Some(10));
    }

    #[tokio::test]
    async fn a_small_file_uploads_in_a_single_chunk_and_captures_the_video_id() {
        let server = MockServer::start().await;
        let upload_path = "/upload/session/one-shot";
        Mock::given(method("PUT"))
            .and(path(upload_path))
            .and(header("Content-Range", "bytes 0-9/10"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "id": "yt-video-123" })),
            )
            .mount(&server)
            .await;

        let uploader = YouTubeUploader::with_base_url(&server.uri(), 8 * 1024 * 1024);
        let dir = temp_dir("youtube-uploader-single-chunk");
        let file_path = write_fake_video(&dir, "clip.mp4", b"0123456789");
        let video = sample_video(&file_path, 10);

        let mut session = UploadSession::new(
            Uuid::nil(),
            Uuid::nil(),
            Platform::YouTube,
            SessionType::Resumable,
        );
        session.remote_upload_url = Some(format!("{}{upload_path}", server.uri()));
        session.bytes_total = Some(10);
        session.state = RemoteUploadState::Initialized;

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let (session, result) = uploader
            .upload_media("access-token", session, &video, tx, CancelSignal::new())
            .await;

        assert!(result.is_ok());
        assert_eq!(session.state, RemoteUploadState::Transferred);
        assert_eq!(session.bytes_committed, 10);
        assert_eq!(session.remote_publish_id.as_deref(), Some("yt-video-123"));
    }

    #[tokio::test]
    async fn expired_session_after_an_uncertain_write_never_restarts_blindly() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path("/upload/session/expired"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        let uploader = YouTubeUploader::with_base_url(&server.uri(), 8 * 1024 * 1024);
        let dir = temp_dir("youtube-uploader-expired-session");
        let file_path = write_fake_video(&dir, "clip.mp4", b"0123456789");
        let video = sample_video(&file_path, 10);
        let mut session = UploadSession::new(
            Uuid::nil(),
            Uuid::nil(),
            Platform::YouTube,
            SessionType::Resumable,
        );
        session.remote_upload_url = Some(format!("{}/upload/session/expired", server.uri()));
        session.bytes_total = Some(10);
        session.state = RemoteUploadState::Transferring;

        assert!(matches!(
            uploader
                .recover_upload("access-token", session, &video)
                .await,
            Err(PublishError::UnknownRemoteResult)
        ));
    }

    #[tokio::test]
    async fn a_429_with_retry_after_is_surfaced_with_the_real_wait_time() {
        let server = MockServer::start().await;
        let upload_path = "/upload/session/rate-limited";
        Mock::given(method("PUT"))
            .and(path(upload_path))
            .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "120"))
            .mount(&server)
            .await;

        let uploader = YouTubeUploader::with_base_url(&server.uri(), 8 * 1024 * 1024);
        let dir = temp_dir("youtube-uploader-rate-limited");
        let file_path = write_fake_video(&dir, "clip.mp4", b"0123456789");
        let video = sample_video(&file_path, 10);

        let mut session = UploadSession::new(
            Uuid::nil(),
            Uuid::nil(),
            Platform::YouTube,
            SessionType::Resumable,
        );
        session.remote_upload_url = Some(format!("{}{upload_path}", server.uri()));
        session.bytes_total = Some(10);
        session.state = RemoteUploadState::Initialized;

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let (_session, result) = uploader
            .upload_media("access-token", session, &video, tx, CancelSignal::new())
            .await;

        match result {
            Err(PublishError::RateLimited {
                retry_after_seconds: Some(120),
            }) => {}
            other => {
                panic!("expected RateLimited{{retry_after_seconds: Some(120)}}, got {other:?}")
            }
        }
    }

    #[tokio::test]
    async fn a_multi_chunk_upload_follows_308_continuation_then_completes() {
        let server = MockServer::start().await;
        let upload_path = "/upload/session/multi-chunk";
        let full_path = upload_path.to_string();

        // 10-byte file, 4-byte chunks: [0-3], [4-7], [8-9] (final).
        Mock::given(method("PUT"))
            .and(path(full_path.clone()))
            .and(header("Content-Range", "bytes 0-3/10"))
            .respond_with(ResponseTemplate::new(308).insert_header("Range", "bytes=0-3"))
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .and(path(full_path.clone()))
            .and(header("Content-Range", "bytes 4-7/10"))
            .respond_with(ResponseTemplate::new(308).insert_header("Range", "bytes=0-7"))
            .mount(&server)
            .await;
        Mock::given(method("PUT"))
            .and(path(full_path))
            .and(header("Content-Range", "bytes 8-9/10"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "id": "yt-video-multi" })),
            )
            .mount(&server)
            .await;

        let uploader = YouTubeUploader::with_base_url(&server.uri(), 4);
        let dir = temp_dir("youtube-uploader-multi-chunk");
        let file_path = write_fake_video(&dir, "clip.mp4", b"0123456789");
        let video = sample_video(&file_path, 10);

        let mut session = UploadSession::new(
            Uuid::nil(),
            Uuid::nil(),
            Platform::YouTube,
            SessionType::Resumable,
        );
        session.remote_upload_url = Some(format!("{}{upload_path}", server.uri()));
        session.bytes_total = Some(10);
        session.state = RemoteUploadState::Initialized;

        let (tx, mut rx): (ProgressSender, _) = tokio::sync::mpsc::unbounded_channel();
        let mut progress_events = Vec::new();
        let progress_task = tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                progress_events.push(event.bytes_uploaded);
            }
            progress_events
        });

        let (session, result) = uploader
            .upload_media("access-token", session, &video, tx, CancelSignal::new())
            .await;

        assert!(result.is_ok(), "{result:?}");
        assert_eq!(session.state, RemoteUploadState::Transferred);
        assert_eq!(session.bytes_committed, 10);
        assert_eq!(session.remote_publish_id.as_deref(), Some("yt-video-multi"));

        let progress = progress_task.await.unwrap();
        assert_eq!(
            progress,
            vec![4, 8, 10],
            "progress should advance with each acknowledged chunk"
        );
    }

    #[tokio::test]
    async fn get_remote_status_maps_processing_states_conservatively() {
        let server = MockServer::start().await;
        for (raw_status, expected) in [
            ("succeeded", RemoteUploadState::RemoteSucceeded),
            ("failed", RemoteUploadState::RemoteFailed),
            ("terminated", RemoteUploadState::RemoteFailed),
            ("processing", RemoteUploadState::RemoteProcessing),
            (
                "something_new_google_added_later",
                RemoteUploadState::RemoteProcessing,
            ),
        ] {
            let video_id = format!("yt-{raw_status}");
            Mock::given(method("GET"))
                .and(path("/youtube/v3/videos"))
                .and(wiremock::matchers::query_param("id", video_id.as_str()))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "items": [{ "id": video_id, "processingDetails": { "processingStatus": raw_status } }]
                })))
                .mount(&server)
                .await;

            let uploader = YouTubeUploader::with_base_url(&server.uri(), 8 * 1024 * 1024);
            let mut session = UploadSession::new(
                Uuid::nil(),
                Uuid::nil(),
                Platform::YouTube,
                SessionType::Resumable,
            );
            session.remote_publish_id = Some(video_id);

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
    async fn recover_upload_queries_actual_committed_bytes_before_resuming() {
        let server = MockServer::start().await;
        let upload_path = "/upload/session/recover";
        let full_url = format!("{}{upload_path}", server.uri());

        // The status-check request: empty body, Content-Range "bytes */10".
        Mock::given(method("PUT"))
            .and(path(upload_path))
            .and(header("Content-Range", "bytes */10"))
            .respond_with(ResponseTemplate::new(308).insert_header("Range", "bytes=0-3"))
            .mount(&server)
            .await;
        // The resumed transfer, starting right after the confirmed range.
        Mock::given(method("PUT"))
            .and(path(upload_path))
            .and(header("Content-Range", "bytes 4-9/10"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "id": "yt-video-recovered" })),
            )
            .mount(&server)
            .await;

        let uploader = YouTubeUploader::with_base_url(&server.uri(), 100);
        let dir = temp_dir("youtube-uploader-recover");
        let file_path = write_fake_video(&dir, "clip.mp4", b"0123456789");
        let video = sample_video(&file_path, 10);

        let mut session = UploadSession::new(
            Uuid::nil(),
            Uuid::nil(),
            Platform::YouTube,
            SessionType::Resumable,
        );
        session.remote_upload_url = Some(full_url);
        session.bytes_total = Some(10);
        session.bytes_committed = 0; // stale/unknown before recovery re-queries
        session.state = RemoteUploadState::Transferring;

        let recovered = uploader
            .recover_upload("access-token", session, &video)
            .await
            .unwrap();

        assert_eq!(recovered.state, RemoteUploadState::Transferred);
        assert_eq!(recovered.bytes_committed, 10);
        assert_eq!(
            recovered.remote_publish_id.as_deref(),
            Some("yt-video-recovered")
        );
    }

    #[test]
    fn validate_metadata_rejects_an_empty_title() {
        let uploader = YouTubeUploader::new();
        let mut metadata = sample_metadata();
        metadata.title = "   ".to_string();
        assert!(uploader.validate_metadata(&metadata).is_err());
    }

    #[test]
    fn validate_metadata_accepts_a_normal_title_and_description() {
        let uploader = YouTubeUploader::new();
        assert!(uploader.validate_metadata(&sample_metadata()).is_ok());
    }
}
