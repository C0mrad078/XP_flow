use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::errors::{DomainError, DomainResult};

/// A `Workspace` is the top-level container for everything a user manages in
/// XP FLOW: channels, videos, publications, templates and settings all live
/// inside exactly one workspace. Phase 1 assumes a single local workspace,
/// but the schema and domain model already scope every child entity by
/// `workspace_id` so multi-workspace support does not require a redesign.
///
/// `timezone` (section 19) is an IANA identifier (e.g. `America/Sao_Paulo`).
/// Every schedule slot's wall-clock time and every calendar view are
/// interpreted in this timezone; every `DateTime<Utc>` persisted to disk
/// stays UTC. Do not rely on the host OS's current timezone anywhere in the
/// scheduler — always resolve through this field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub timezone: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Workspace {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            timezone: "UTC".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Validates an IANA timezone identifier against the compiled tz
    /// database, rejecting typos/garbage up front instead of failing later
    /// deep inside a scheduling calculation.
    pub fn validate_timezone(tz: &str) -> DomainResult<()> {
        tz.parse::<chrono_tz::Tz>()
            .map(|_| ())
            .map_err(|_| DomainError::InvalidValue {
                field: "timezone",
                reason: format!("{tz:?} is not a recognized IANA timezone"),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_workspace_defaults_to_utc() {
        assert_eq!(Workspace::new("Test").timezone, "UTC");
    }

    #[test]
    fn accepts_known_iana_zones() {
        assert!(Workspace::validate_timezone("America/Sao_Paulo").is_ok());
        assert!(Workspace::validate_timezone("UTC").is_ok());
        assert!(Workspace::validate_timezone("Asia/Tokyo").is_ok());
    }

    #[test]
    fn rejects_unknown_zones() {
        assert!(Workspace::validate_timezone("Not/A_Zone").is_err());
        assert!(Workspace::validate_timezone("").is_err());
    }
}
