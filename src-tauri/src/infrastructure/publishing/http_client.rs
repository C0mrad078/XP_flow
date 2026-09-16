//! Section 114: upload traffic needs its own timeout policy, entirely
//! separate from `infrastructure::auth::http_client`'s short
//! identity-call timeouts — a multi-megabyte chunk PUT over a slow
//! connection legitimately takes longer than any OAuth call should.

use std::time::Duration;

/// A single chunk PUT — generous for even a slow connection on a
/// multi-megabyte chunk, but still bounded (never an unbounded hang).
pub const UPLOAD_CHUNK_TIMEOUT: Duration = Duration::from_secs(180);
/// Session initialization / finalize calls — small JSON bodies, no media.
pub const UPLOAD_CONTROL_TIMEOUT: Duration = Duration::from_secs(20);
/// Remote processing status polls — small JSON responses.
pub const PROCESSING_STATUS_TIMEOUT: Duration = Duration::from_secs(20);

/// Every provider uploader builds its HTTP client through here (section
/// 115: reuse connection pools, never construct a fresh `reqwest::Client`
/// per chunk) — no default request timeout is set since different calls
/// need different bounds; callers apply `.timeout(...)` per request from
/// the constants above.
pub fn build_upload_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .build()
        .expect("building the upload HTTP client with static configuration cannot fail")
}
