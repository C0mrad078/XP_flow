use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    #[default]
    Dark,
    Light,
    System,
}

impl ThemePreference {
    pub fn as_str(&self) -> &'static str {
        match self {
            ThemePreference::Dark => "dark",
            ThemePreference::Light => "light",
            ThemePreference::System => "system",
        }
    }
}

impl std::str::FromStr for ThemePreference {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dark" => Ok(ThemePreference::Dark),
            "light" => Ok(ThemePreference::Light),
            "system" => Ok(ThemePreference::System),
            other => Err(format!("unknown theme preference: {other}")),
        }
    }
}

/// Application-wide settings. Persisted as a single key/value table
/// (`app_settings`) so future settings can be added without a migration for
/// every new field; this struct is the typed view the frontend receives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: ThemePreference,
    pub launch_on_startup: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::Dark,
            launch_on_startup: false,
        }
    }
}
