use std::path::Path;

use tauri::http::{Request, Response, StatusCode};
use tauri::{AppHandle, Manager};
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use uuid::Uuid;

use crate::state::AppState;

/// The scheme local video preview and thumbnails are served under
/// (section 42): the frontend never receives a filesystem path, only
/// `https://xpflowmedia.localhost/video/<id>` or `.../thumbnail/<id>` —
/// this handler resolves the id to a path via the database and streams
/// bytes back, with HTTP Range support on the video route so `<video>`
/// seeking works without loading a whole 4K file into memory (section 88).
pub const MEDIA_PROTOCOL_SCHEME: &str = "xpflowmedia";

pub fn register(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
    builder.register_asynchronous_uri_scheme_protocol(
        MEDIA_PROTOCOL_SCHEME,
        move |ctx, request, responder| {
            let app_handle = ctx.app_handle().clone();
            tauri::async_runtime::spawn(async move {
                let response = handle_request(&app_handle, request).await;
                responder.respond(response);
            });
        },
    )
}

async fn handle_request(app: &AppHandle, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let Some((kind, video_id)) = parse_route(&request) else {
        return not_found();
    };

    let state = app.state::<AppState>();
    let video = match state.video_repo.get(video_id).await {
        Ok(Some(video)) => video,
        _ => return not_found(),
    };

    let file_path = match kind {
        RequestKind::Video => Some(video.file_path.clone()),
        RequestKind::Thumbnail => video.thumbnail_path.clone(),
    };
    let Some(file_path) = file_path else {
        return not_found();
    };
    let path = Path::new(&file_path);
    let mime = mime_guess::from_path(path).first_or_octet_stream();

    let Ok(mut file) = tokio::fs::File::open(path).await else {
        return not_found();
    };
    let Ok(metadata) = file.metadata().await else {
        return not_found();
    };
    let file_size = metadata.len();

    let range_header = request.headers().get("range").and_then(|v| v.to_str().ok());

    match range_header.and_then(|r| parse_range(r, file_size)) {
        Some((start, end)) => {
            let length = end - start + 1;
            if file.seek(std::io::SeekFrom::Start(start)).await.is_err() {
                return not_found();
            }
            let mut buffer = vec![0u8; length as usize];
            if file.read_exact(&mut buffer).await.is_err() {
                return not_found();
            }

            Response::builder()
                .status(StatusCode::PARTIAL_CONTENT)
                .header("Content-Type", mime.as_ref())
                .header("Accept-Ranges", "bytes")
                .header("Content-Range", format!("bytes {start}-{end}/{file_size}"))
                .header("Content-Length", length.to_string())
                .body(buffer)
                .unwrap_or_else(|_| not_found())
        }
        None => {
            let mut buffer = Vec::with_capacity(file_size as usize);
            if file.read_to_end(&mut buffer).await.is_err() {
                return not_found();
            }

            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", mime.as_ref())
                .header("Accept-Ranges", "bytes")
                .header("Content-Length", buffer.len().to_string())
                .body(buffer)
                .unwrap_or_else(|_| not_found())
        }
    }
}

enum RequestKind {
    Video,
    Thumbnail,
}

fn parse_route(request: &Request<Vec<u8>>) -> Option<(RequestKind, Uuid)> {
    let path = request.uri().path().trim_start_matches('/');
    let (kind_str, id_str) = path.split_once('/')?;
    let kind = match kind_str {
        "video" => RequestKind::Video,
        "thumbnail" => RequestKind::Thumbnail,
        _ => return None,
    };
    let id = Uuid::parse_str(id_str).ok()?;
    Some((kind, id))
}

/// Parses a single-range `Range: bytes=start-end` header (also handling
/// the open-ended `bytes=start-` and suffix `bytes=-N` forms). Multi-range
/// requests are not supported — no browser/webview media engine sends
/// them for `<video>` playback.
fn parse_range(header: &str, file_size: u64) -> Option<(u64, u64)> {
    let spec = header.strip_prefix("bytes=")?;
    let (start_str, end_str) = spec.split_once('-')?;

    if start_str.is_empty() {
        let suffix_len: u64 = end_str.parse().ok()?;
        let start = file_size.saturating_sub(suffix_len);
        return Some((start, file_size.saturating_sub(1)));
    }

    let start: u64 = start_str.parse().ok()?;
    let end = if end_str.is_empty() {
        file_size.saturating_sub(1)
    } else {
        end_str.parse().ok()?
    };

    if start > end || end >= file_size {
        return None;
    }

    Some((start, end))
}

fn not_found() -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Vec::new())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_bounded_range() {
        assert_eq!(parse_range("bytes=100-199", 1000), Some((100, 199)));
    }

    #[test]
    fn parses_an_open_ended_range() {
        assert_eq!(parse_range("bytes=900-", 1000), Some((900, 999)));
    }

    #[test]
    fn parses_a_suffix_range() {
        assert_eq!(parse_range("bytes=-100", 1000), Some((900, 999)));
    }

    #[test]
    fn rejects_a_range_past_the_end_of_file() {
        assert_eq!(parse_range("bytes=900-1500", 1000), None);
    }

    #[test]
    fn rejects_malformed_headers() {
        assert_eq!(parse_range("not-a-range-header", 1000), None);
        assert_eq!(parse_range("bytes=abc-def", 1000), None);
    }
}
