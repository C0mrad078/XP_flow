use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::domain::analytics::{
    AnalyticsAvailability, AnalyticsCapabilities, PublicationMetricSnapshot,
};
use crate::domain::platform::Platform;
use crate::domain::ports::analytics_provider::AnalyticsProvider;
use crate::domain::publishing::PublishError;

#[derive(Deserialize)]
struct Response {
    items: Vec<Item>,
}
#[derive(Deserialize)]
struct Item {
    statistics: Option<Stats>,
}
#[derive(Deserialize)]
struct Stats {
    #[serde(rename = "viewCount")]
    views: Option<String>,
    #[serde(rename = "likeCount")]
    likes: Option<String>,
    #[serde(rename = "commentCount")]
    comments: Option<String>,
}

fn parse_count(value: Option<String>) -> Option<i64> {
    value.and_then(|value| value.parse().ok())
}

pub struct YouTubeAnalyticsProvider {
    http: reqwest::Client,
    endpoint: String,
}
impl YouTubeAnalyticsProvider {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
            endpoint: "https://www.googleapis.com/youtube/v3/videos".into(),
        }
    }
}

impl Default for YouTubeAnalyticsProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AnalyticsProvider for YouTubeAnalyticsProvider {
    fn capabilities(&self) -> AnalyticsCapabilities {
        AnalyticsCapabilities {
            platform: Platform::YouTube,
            publication_views: true,
            publication_likes: true,
            publication_comments: true,
            publication_shares: false,
            channel_followers: false,
        }
    }
    async fn fetch_publication_metrics(
        &self,
        token: &str,
        remote_id: &str,
    ) -> Result<PublicationMetricSnapshot, PublishError> {
        let response = self
            .http
            .get(&self.endpoint)
            .bearer_auth(token)
            .query(&[("part", "statistics"), ("id", remote_id)])
            .send()
            .await
            .map_err(|e| PublishError::Internal {
                detail: e.to_string(),
            })?;
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(PublishError::RateLimited {
                retry_after_seconds: None,
            });
        }
        if !response.status().is_success() {
            return Err(PublishError::Internal {
                detail: format!(
                    "YouTube analytics request failed with {}",
                    response.status()
                ),
            });
        }
        let body: Response = response.json().await.map_err(|e| PublishError::Internal {
            detail: e.to_string(),
        })?;
        let Some(item) = body.items.into_iter().next() else {
            return Ok(PublicationMetricSnapshot {
                id: Uuid::new_v4(),
                publication_id: Uuid::nil(),
                provider: Platform::YouTube,
                captured_at: Utc::now(),
                views: None,
                likes: None,
                comments: None,
                shares: None,
                availability: AnalyticsAvailability::Unavailable,
                error_code: Some("REMOTE_NOT_FOUND".into()),
            });
        };
        let stats = item.statistics;
        Ok(PublicationMetricSnapshot {
            id: Uuid::new_v4(),
            publication_id: Uuid::nil(),
            provider: Platform::YouTube,
            captured_at: Utc::now(),
            views: stats.as_ref().and_then(|s| parse_count(s.views.clone())),
            likes: stats.as_ref().and_then(|s| parse_count(s.likes.clone())),
            comments: stats.as_ref().and_then(|s| parse_count(s.comments.clone())),
            shares: None,
            availability: AnalyticsAvailability::Supported,
            error_code: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::parse_count;

    #[test]
    fn provider_counts_preserve_zero_and_reject_malformed_values() {
        assert_eq!(parse_count(Some("0".into())), Some(0));
        assert_eq!(parse_count(Some("42".into())), Some(42));
        assert_eq!(parse_count(Some("not-a-count".into())), None);
        assert_eq!(parse_count(None), None);
    }
}
