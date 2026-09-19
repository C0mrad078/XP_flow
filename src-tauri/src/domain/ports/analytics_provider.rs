use async_trait::async_trait;

use crate::domain::analytics::{AnalyticsCapabilities, PublicationMetricSnapshot};
use crate::domain::platform::Platform;
use crate::domain::publishing::PublishError;

#[async_trait]
pub trait AnalyticsProvider: Send + Sync {
    fn capabilities(&self) -> AnalyticsCapabilities;
    async fn fetch_publication_metrics(
        &self,
        access_token: &str,
        remote_id: &str,
    ) -> Result<PublicationMetricSnapshot, PublishError>;
}

pub fn unsupported_capabilities(platform: Platform) -> AnalyticsCapabilities {
    AnalyticsCapabilities {
        platform,
        publication_views: false,
        publication_likes: false,
        publication_comments: false,
        publication_shares: false,
        channel_followers: false,
    }
}
