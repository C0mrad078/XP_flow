/// Where the desktop finds the Auth Broker (section 28). Never
/// hardcoded — resolved once at startup from environment/build
/// configuration.
#[derive(Debug, Clone)]
pub enum AuthBrokerConfig {
    /// Local development broker — plain HTTP to loopback only.
    Development { base_url: String },
    /// A production deployment — HTTPS is mandatory (section 83).
    Production { base_url: String },
    /// An explicitly-provided URL (e.g. staging) — same HTTPS rule as
    /// Production unless it points at loopback, which is treated as a
    /// development convenience.
    Custom { base_url: String },
}

impl AuthBrokerConfig {
    /// Resolves configuration from the environment. `XPFLOW_AUTH_BROKER_URL`
    /// wins if set (Custom); otherwise debug builds default to a local
    /// broker on `127.0.0.1:8787` (Development) and release builds are
    /// unconfigured (`None`) until an operator sets the env var — a
    /// missing broker must degrade the affected providers, never crash
    /// the app (section 65/80).
    pub fn resolve() -> Option<Self> {
        if let Ok(url) = std::env::var("XPFLOW_AUTH_BROKER_URL") {
            let trimmed = url.trim();
            if !trimmed.is_empty() {
                return Some(AuthBrokerConfig::Custom {
                    base_url: trimmed.trim_end_matches('/').to_string(),
                });
            }
        }
        if cfg!(debug_assertions) {
            return Some(AuthBrokerConfig::Development {
                base_url: "http://127.0.0.1:8787".to_string(),
            });
        }
        None
    }

    pub fn base_url(&self) -> &str {
        match self {
            AuthBrokerConfig::Development { base_url }
            | AuthBrokerConfig::Production { base_url }
            | AuthBrokerConfig::Custom { base_url } => base_url,
        }
    }

    /// Section 83: no production configuration may point at plaintext
    /// HTTP unless it's explicitly loopback (a developer's own machine).
    pub fn is_secure_enough(&self) -> bool {
        let url = self.base_url();
        if url.starts_with("https://") {
            return true;
        }
        url.starts_with("http://127.0.0.1") || url.starts_with("http://localhost")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_http_is_considered_secure_enough_for_development() {
        let config = AuthBrokerConfig::Development {
            base_url: "http://127.0.0.1:8787".to_string(),
        };
        assert!(config.is_secure_enough());
    }

    #[test]
    fn a_non_loopback_plaintext_url_is_rejected() {
        let config = AuthBrokerConfig::Production {
            base_url: "http://auth.example.com".to_string(),
        };
        assert!(!config.is_secure_enough());
    }

    #[test]
    fn https_is_always_accepted() {
        let config = AuthBrokerConfig::Production {
            base_url: "https://auth.example.com".to_string(),
        };
        assert!(config.is_secure_enough());
    }
}
