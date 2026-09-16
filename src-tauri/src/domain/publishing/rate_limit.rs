use serde::{Deserialize, Serialize};

/// Which category of provider request a rate-limit window applies to
/// (Phase 5.1 section 17). Fully wired for `Publish`/`Status`/`Auth`;
/// `Comments`/`Analytics` are declared now so the same vocabulary is
/// ready when Phase 6 adds those operations, but nothing in this phase
/// ever records or checks them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RateLimitOperation {
    Publish,
    Status,
    Auth,
    Comments,
    Analytics,
}

impl RateLimitOperation {
    pub fn as_str(&self) -> &'static str {
        match self {
            RateLimitOperation::Publish => "publish",
            RateLimitOperation::Status => "status",
            RateLimitOperation::Auth => "auth",
            RateLimitOperation::Comments => "comments",
            RateLimitOperation::Analytics => "analytics",
        }
    }
}

impl std::fmt::Display for RateLimitOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for RateLimitOperation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "publish" => RateLimitOperation::Publish,
            "status" => RateLimitOperation::Status,
            "auth" => RateLimitOperation::Auth,
            "comments" => RateLimitOperation::Comments,
            "analytics" => RateLimitOperation::Analytics,
            other => return Err(format!("unknown rate limit operation: {other}")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_string() {
        for op in [
            RateLimitOperation::Publish,
            RateLimitOperation::Status,
            RateLimitOperation::Auth,
            RateLimitOperation::Comments,
            RateLimitOperation::Analytics,
        ] {
            assert_eq!(op.as_str().parse::<RateLimitOperation>().unwrap(), op);
        }
    }
}
