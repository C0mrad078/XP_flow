use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;

/// Cryptographically random anti-forgery state token (section 12) — the
/// broker generates its own for the Kwai flow, which it owns end to end.
pub fn generate_state() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Least-privilege default scopes (section 8/15) for the one brokered
/// provider that needs the broker to build its own authorize URL (Kwai —
/// TikTok's authorize URL is built entirely desktop-side).
pub fn default_scopes_for(platform: &str) -> Vec<String> {
    match platform {
        "kwai" => vec!["user_info".to_string()],
        _ => Vec::new(),
    }
}
