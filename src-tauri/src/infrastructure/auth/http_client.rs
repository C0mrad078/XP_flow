use std::time::Duration;

/// Timeout for a normal OAuth/identity HTTP call (token exchange, refresh,
/// profile fetch). Deliberately short — section 67: "OAuth identity
/// validation should not use video-upload-scale timeouts."
pub const AUTH_HTTP_TIMEOUT: Duration = Duration::from_secs(15);

/// Timeout for the desktop <-> Auth Broker polling call specifically —
/// short and cheap since it's called repeatedly while a session is open.
pub const BROKER_POLL_TIMEOUT: Duration = Duration::from_secs(8);

/// Every outbound HTTP call in the auth subsystem goes through a client
/// built here (section 66) — never a bare `reqwest::get`/scattered
/// per-call client construction, so the timeout policy can't accidentally
/// be forgotten at one call site.
pub fn build_auth_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(AUTH_HTTP_TIMEOUT)
        .connect_timeout(Duration::from_secs(5))
        .build()
        .expect("building the auth HTTP client with static configuration cannot fail")
}
