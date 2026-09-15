use serde::{Deserialize, Serialize};

/// Live progress of an in-flight authorization attempt (section 44),
/// pushed to the frontend so the "Connecting…" dialog can show something
/// more useful than a spinner. Distinct from `PlatformAccountStatus`,
/// which describes the *persisted* account, not a live in-progress flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AuthFlowState {
    OpeningBrowser,
    WaitingForAuthorization,
    VerifyingAccount,
    SavingConnection,
    Connected,
    Failed { code: String, message: String },
}
