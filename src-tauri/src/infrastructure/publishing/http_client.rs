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

/// Parses a `Retry-After` response header's delay-seconds form (RFC 9110
/// §10.2.3) — the form every provider this codebase talks to actually
/// sends. The header's alternative HTTP-date form is not implemented (no
/// provider observed here uses it); an unparseable or absent header
/// yields `None` rather than a guess, which callers already treat as "no
/// provider-supplied timing, fall back to the generic retry policy"
/// (Phase 5.1 section 18/20).
pub fn parse_retry_after_seconds(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_plain_delay_seconds_header() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::RETRY_AFTER, "120".parse().unwrap());
        assert_eq!(parse_retry_after_seconds(&headers), Some(120));
    }

    #[test]
    fn returns_none_when_the_header_is_absent() {
        let headers = reqwest::header::HeaderMap::new();
        assert_eq!(parse_retry_after_seconds(&headers), None);
    }

    #[test]
    fn returns_none_for_an_http_date_form_it_does_not_parse() {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::RETRY_AFTER,
            "Wed, 21 Oct 2026 07:28:00 GMT".parse().unwrap(),
        );
        assert_eq!(parse_retry_after_seconds(&headers), None);
    }
}
