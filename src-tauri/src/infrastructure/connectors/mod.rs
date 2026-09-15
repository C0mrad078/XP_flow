//! Stub `PlatformConnector` adapter. Every method returns
//! `PlatformConnectorError::NotImplemented` — there is no real network
//! call, no OAuth flow, no API client yet. Its purpose is to prove the
//! `PlatformConnector` port (domain::ports::platform_connector) is actually
//! satisfiable by all three platforms before Phase 2 fills in a real
//! implementation per platform, and to give the Channels UI a stable
//! contract to build against today.

use async_trait::async_trait;

use crate::domain::platform::Platform;
use crate::domain::ports::platform_connector::{PlatformConnector, PlatformConnectorError};
use crate::domain::publication::Publication;

pub struct StubConnector {
    platform: Platform,
}

impl StubConnector {
    pub fn new(platform: Platform) -> Self {
        Self { platform }
    }

    fn not_implemented<T>(&self) -> Result<T, PlatformConnectorError> {
        Err(PlatformConnectorError::NotImplemented {
            platform: self.platform,
        })
    }
}

#[async_trait]
impl PlatformConnector for StubConnector {
    fn platform(&self) -> Platform {
        self.platform
    }

    async fn authenticate(&self) -> Result<String, PlatformConnectorError> {
        self.not_implemented()
    }

    async fn disconnect(&self, _external_account_id: &str) -> Result<(), PlatformConnectorError> {
        self.not_implemented()
    }

    async fn validate_session(
        &self,
        _external_account_id: &str,
    ) -> Result<bool, PlatformConnectorError> {
        self.not_implemented()
    }

    async fn publish_video(
        &self,
        _publication: &Publication,
    ) -> Result<String, PlatformConnectorError> {
        self.not_implemented()
    }

    async fn get_publication_status(
        &self,
        _remote_id: &str,
    ) -> Result<String, PlatformConnectorError> {
        self.not_implemented()
    }

    async fn fetch_metrics(
        &self,
        _remote_id: &str,
    ) -> Result<serde_json::Value, PlatformConnectorError> {
        self.not_implemented()
    }

    async fn fetch_comments(
        &self,
        _remote_id: &str,
    ) -> Result<serde_json::Value, PlatformConnectorError> {
        self.not_implemented()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn stub_connector_reports_not_implemented() {
        for platform in Platform::ALL {
            let connector = StubConnector::new(platform);
            assert_eq!(connector.platform(), platform);
            let err = connector.authenticate().await.unwrap_err();
            assert!(matches!(err, PlatformConnectorError::NotImplemented { .. }));
        }
    }
}
