use std::sync::Arc;

use crate::config::Config;
use crate::crypto::Cipher;
use crate::providers::kwai::KwaiProviderClient;
use crate::providers::tiktok::TikTokProviderClient;
use crate::store::Store;

/// Everything a route handler needs, assembled once at startup. Mirrors
/// the desktop's own `AppState` pattern deliberately — same shape,
/// completely separate instance, no shared code between the two crates.
pub struct AppState {
    pub store: Store,
    pub cipher: Cipher,
    pub tiktok: Option<Arc<TikTokProviderClient>>,
    pub kwai: Option<Arc<KwaiProviderClient>>,
    pub public_base_url: String,
}

impl AppState {
    pub fn new(config: Config, store: Store) -> Self {
        Self {
            store,
            cipher: Cipher::new(config.master_key),
            tiktok: config
                .tiktok
                .map(|c| Arc::new(TikTokProviderClient::new(c))),
            kwai: config.kwai.map(|c| Arc::new(KwaiProviderClient::new(c))),
            public_base_url: config.public_base_url,
        }
    }

    /// Assembles state from already-built parts — used by integration
    /// tests to inject provider clients pointed at a local mock server
    /// (`TikTokProviderClient::with_base_url`/`KwaiProviderClient::with_base_url`)
    /// instead of the real hardcoded provider endpoints (section 90).
    pub fn from_parts(
        store: Store,
        master_key: [u8; 32],
        tiktok: Option<Arc<TikTokProviderClient>>,
        kwai: Option<Arc<KwaiProviderClient>>,
        public_base_url: String,
    ) -> Self {
        Self {
            store,
            cipher: Cipher::new(master_key),
            tiktok,
            kwai,
            public_base_url,
        }
    }
}
