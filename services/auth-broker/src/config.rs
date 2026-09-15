use std::env;

/// All broker configuration comes from the environment (section 27) —
/// never a config file with secrets baked in, never a hardcoded default
/// for anything confidential.
#[derive(Clone)]
pub struct Config {
    pub bind_addr: String,
    pub database_url: String,
    /// Raw 32-byte AES-256-GCM key, base64-encoded in the environment.
    pub master_key: [u8; 32],
    pub tiktok: Option<TikTokCredentials>,
    pub kwai: Option<KwaiCredentials>,
    /// This broker instance's own public base URL — used to build the
    /// Kwai callback URI it registers as its redirect (section 18).
    pub public_base_url: String,
}

#[derive(Clone)]
pub struct TikTokCredentials {
    pub client_key: String,
    pub client_secret: String,
}

#[derive(Clone)]
pub struct KwaiCredentials {
    pub app_id: String,
    pub app_secret: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    Missing(&'static str),
    #[error("AUTH_BROKER_MASTER_KEY must be exactly 32 bytes once base64-decoded (got {0} bytes) — see .env.example")]
    InvalidMasterKeyLength(usize),
    #[error("AUTH_BROKER_MASTER_KEY is not valid base64: {0}")]
    InvalidMasterKeyEncoding(String),
}

impl Config {
    /// Fails fast (section 85) — unlike the desktop app, which must
    /// degrade a single unconfigured provider gracefully, the broker's
    /// entire reason to exist is encrypting/exchanging credentials; it
    /// cannot usefully run at all without a valid master key.
    pub fn from_env() -> Result<Self, ConfigError> {
        let master_key = decode_master_key(&require_env("AUTH_BROKER_MASTER_KEY")?)?;

        let tiktok = match (
            env::var("TIKTOK_CLIENT_KEY"),
            env::var("TIKTOK_CLIENT_SECRET"),
        ) {
            (Ok(client_key), Ok(client_secret))
                if !client_key.trim().is_empty() && !client_secret.trim().is_empty() =>
            {
                Some(TikTokCredentials {
                    client_key,
                    client_secret,
                })
            }
            _ => None,
        };
        let kwai = match (env::var("KWAI_APP_ID"), env::var("KWAI_APP_SECRET")) {
            (Ok(app_id), Ok(app_secret))
                if !app_id.trim().is_empty() && !app_secret.trim().is_empty() =>
            {
                Some(KwaiCredentials { app_id, app_secret })
            }
            _ => None,
        };

        Ok(Self {
            bind_addr: env::var("AUTH_BROKER_BIND_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:8787".to_string()),
            database_url: env::var("AUTH_BROKER_DATABASE_PATH")
                .unwrap_or_else(|_| "auth-broker.db".to_string()),
            master_key,
            tiktok,
            kwai,
            public_base_url: env::var("AUTH_BROKER_PUBLIC_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8787".to_string()),
        })
    }
}

fn require_env(key: &'static str) -> Result<String, ConfigError> {
    env::var(key).map_err(|_| ConfigError::Missing(key))
}

fn decode_master_key(encoded: &str) -> Result<[u8; 32], ConfigError> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    let bytes = STANDARD
        .decode(encoded.trim())
        .map_err(|e| ConfigError::InvalidMasterKeyEncoding(e.to_string()))?;
    if bytes.len() != 32 {
        return Err(ConfigError::InvalidMasterKeyLength(bytes.len()));
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;

    #[test]
    fn rejects_a_master_key_of_the_wrong_length() {
        let short_key = STANDARD.encode([1u8; 16]);
        assert!(matches!(
            decode_master_key(&short_key),
            Err(ConfigError::InvalidMasterKeyLength(16))
        ));
    }

    #[test]
    fn accepts_a_valid_32_byte_key() {
        let key = STANDARD.encode([7u8; 32]);
        assert!(decode_master_key(&key).is_ok());
    }

    #[test]
    fn rejects_non_base64_input() {
        assert!(decode_master_key("not-valid-base64!!!").is_err());
    }
}
