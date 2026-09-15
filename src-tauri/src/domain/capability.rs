use serde::{Deserialize, Serialize};

use super::platform::Platform;

/// XP FLOW-level permissions, independent of any provider's own scope
/// naming (section 33/34). A `PlatformAccount` can be `Connected` while
/// still lacking `UploadVideo` — the UI reasons about capabilities, never
/// raw provider scope strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    ReadProfile,
    UploadVideo,
    ReadVideoStatus,
    ReadMetrics,
    ReadComments,
    WriteComments,
}

impl Capability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Capability::ReadProfile => "read_profile",
            Capability::UploadVideo => "upload_video",
            Capability::ReadVideoStatus => "read_video_status",
            Capability::ReadMetrics => "read_metrics",
            Capability::ReadComments => "read_comments",
            Capability::WriteComments => "write_comments",
        }
    }
}

impl std::str::FromStr for Capability {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "read_profile" => Capability::ReadProfile,
            "upload_video" => Capability::UploadVideo,
            "read_video_status" => Capability::ReadVideoStatus,
            "read_metrics" => Capability::ReadMetrics,
            "read_comments" => Capability::ReadComments,
            "write_comments" => Capability::WriteComments,
            other => return Err(format!("unknown capability: {other}")),
        })
    }
}

/// Maps a set of provider-granted OAuth scope strings to XP FLOW
/// capabilities (section 34's "provider grants some scope -> connector
/// reports UPLOAD_VIDEO capability"). Pure and provider-specific — this is
/// the single place that ever has to know what a raw scope string means.
///
/// Phase 4 only ever *requests* read/identity scopes (least privilege,
/// section 8/15) — the upload/publish mappings exist now so Phase 5 can
/// request the additional scope and have it "just work" without touching
/// this function's shape, only its data.
pub fn map_scopes_to_capabilities(platform: Platform, scopes: &[String]) -> Vec<Capability> {
    let has = |scope: &str| scopes.iter().any(|s| s == scope);
    let mut capabilities = Vec::new();

    match platform {
        Platform::YouTube => {
            if has("openid") || has("https://www.googleapis.com/auth/userinfo.profile") {
                capabilities.push(Capability::ReadProfile);
            }
            if has("https://www.googleapis.com/auth/youtube.readonly")
                || has("https://www.googleapis.com/auth/youtube.force-ssl")
            {
                capabilities.push(Capability::ReadVideoStatus);
                capabilities.push(Capability::ReadMetrics);
            }
            if has("https://www.googleapis.com/auth/youtube.upload")
                || has("https://www.googleapis.com/auth/youtube.force-ssl")
            {
                capabilities.push(Capability::UploadVideo);
            }
            if has("https://www.googleapis.com/auth/youtube.force-ssl") {
                capabilities.push(Capability::ReadComments);
                capabilities.push(Capability::WriteComments);
            }
        }
        Platform::TikTok => {
            if has("user.info.basic") || has("user.info.profile") {
                capabilities.push(Capability::ReadProfile);
            }
            if has("video.list") {
                capabilities.push(Capability::ReadVideoStatus);
            }
            if has("video.publish") || has("video.upload") {
                capabilities.push(Capability::UploadVideo);
            }
        }
        Platform::Kwai => {
            if has("user_info") {
                capabilities.push(Capability::ReadProfile);
            }
            if has("video_upload") {
                capabilities.push(Capability::UploadVideo);
            }
            if has("video_status") {
                capabilities.push(Capability::ReadVideoStatus);
            }
        }
    }

    capabilities.sort_by_key(|c| c.as_str());
    capabilities.dedup();
    capabilities
}

/// The minimal scope set Phase 4 requests per provider — identity/read
/// only, never publishing (section 8/15's least-privilege requirement).
pub fn default_requested_scopes(platform: Platform) -> Vec<String> {
    match platform {
        Platform::YouTube => vec![
            "openid".to_string(),
            "https://www.googleapis.com/auth/userinfo.profile".to_string(),
            "https://www.googleapis.com/auth/youtube.readonly".to_string(),
        ],
        Platform::TikTok => vec!["user.info.basic".to_string()],
        Platform::Kwai => vec!["user_info".to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn youtube_readonly_grants_read_capabilities_but_not_upload() {
        let scopes = vec![
            "openid".to_string(),
            "https://www.googleapis.com/auth/youtube.readonly".to_string(),
        ];
        let capabilities = map_scopes_to_capabilities(Platform::YouTube, &scopes);
        assert!(capabilities.contains(&Capability::ReadProfile));
        assert!(capabilities.contains(&Capability::ReadVideoStatus));
        assert!(!capabilities.contains(&Capability::UploadVideo));
    }

    #[test]
    fn youtube_upload_scope_grants_upload_capability() {
        let scopes = vec!["https://www.googleapis.com/auth/youtube.upload".to_string()];
        let capabilities = map_scopes_to_capabilities(Platform::YouTube, &scopes);
        assert!(capabilities.contains(&Capability::UploadVideo));
    }

    #[test]
    fn tiktok_basic_info_only_grants_profile_read() {
        let scopes = vec!["user.info.basic".to_string()];
        let capabilities = map_scopes_to_capabilities(Platform::TikTok, &scopes);
        assert_eq!(capabilities, vec![Capability::ReadProfile]);
    }

    #[test]
    fn kwai_video_upload_scope_grants_upload_capability() {
        let scopes = vec!["user_info".to_string(), "video_upload".to_string()];
        let capabilities = map_scopes_to_capabilities(Platform::Kwai, &scopes);
        assert!(capabilities.contains(&Capability::ReadProfile));
        assert!(capabilities.contains(&Capability::UploadVideo));
    }

    #[test]
    fn unknown_scopes_grant_nothing() {
        assert!(
            map_scopes_to_capabilities(Platform::YouTube, &["nonsense.scope".to_string()])
                .is_empty()
        );
    }

    #[test]
    fn default_requested_scopes_never_include_publishing() {
        for platform in Platform::ALL {
            let scopes = default_requested_scopes(platform);
            let capabilities = map_scopes_to_capabilities(platform, &scopes);
            assert!(
                !capabilities.contains(&Capability::UploadVideo),
                "{platform:?} default scopes must stay least-privilege in Phase 4"
            );
        }
    }
}
