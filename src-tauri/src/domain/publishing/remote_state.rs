use serde::{Deserialize, Serialize};

/// A provider-agnostic view of where an upload actually stands with the
/// remote platform (section 64) — deliberately separate from
/// [`super::super::publication::PublicationStatus`], which is the
/// coarser, user-facing state XP FLOW has used since Phase 3.
///
/// The distinction matters for recovery: after a crash, "the Publication
/// was `Uploading`" doesn't tell you whether the provider ever received
/// any bytes. `RemoteUploadState` is what the crash-recovery pass
/// actually inspects (section 65/88).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoteUploadState {
    NotStarted,
    Initialized,
    Transferring,
    Transferred,
    RemoteProcessing,
    RemoteSucceeded,
    RemoteFailed,
    /// The outcome of the last remote request is genuinely unknown — a
    /// timeout after the provider may have already accepted the write
    /// (section 43/69). This is not the same as `RemoteFailed`: retrying
    /// from this state without first verifying is exactly how a
    /// duplicate remote post gets created.
    RemoteUnknown,
}

impl RemoteUploadState {
    pub fn as_str(&self) -> &'static str {
        match self {
            RemoteUploadState::NotStarted => "not_started",
            RemoteUploadState::Initialized => "initialized",
            RemoteUploadState::Transferring => "transferring",
            RemoteUploadState::Transferred => "transferred",
            RemoteUploadState::RemoteProcessing => "remote_processing",
            RemoteUploadState::RemoteSucceeded => "remote_succeeded",
            RemoteUploadState::RemoteFailed => "remote_failed",
            RemoteUploadState::RemoteUnknown => "remote_unknown",
        }
    }

    /// Whether a *new* upload session may be safely initialized from this
    /// state. `Transferred`/`RemoteProcessing`/`RemoteSucceeded`/
    /// `RemoteUnknown` all mean "a remote write may already exist" —
    /// starting a fresh session from any of those risks a duplicate post
    /// (section 3/66). Only a state that never resulted in a provider
    /// accepting a write is safe to restart from scratch.
    pub fn safe_to_restart(&self) -> bool {
        matches!(
            self,
            RemoteUploadState::NotStarted
                | RemoteUploadState::Initialized
                | RemoteUploadState::RemoteFailed
        )
    }
}

impl std::fmt::Display for RemoteUploadState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for RemoteUploadState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "not_started" => RemoteUploadState::NotStarted,
            "initialized" => RemoteUploadState::Initialized,
            "transferring" => RemoteUploadState::Transferring,
            "transferred" => RemoteUploadState::Transferred,
            "remote_processing" => RemoteUploadState::RemoteProcessing,
            "remote_succeeded" => RemoteUploadState::RemoteSucceeded,
            "remote_failed" => RemoteUploadState::RemoteFailed,
            "remote_unknown" => RemoteUploadState::RemoteUnknown,
            other => return Err(format!("unknown remote upload state: {other}")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_never_written_states_are_safe_to_restart() {
        assert!(RemoteUploadState::NotStarted.safe_to_restart());
        assert!(RemoteUploadState::Initialized.safe_to_restart());
        assert!(RemoteUploadState::RemoteFailed.safe_to_restart());
        assert!(!RemoteUploadState::Transferring.safe_to_restart());
        assert!(!RemoteUploadState::Transferred.safe_to_restart());
        assert!(!RemoteUploadState::RemoteProcessing.safe_to_restart());
        assert!(!RemoteUploadState::RemoteSucceeded.safe_to_restart());
        assert!(!RemoteUploadState::RemoteUnknown.safe_to_restart());
    }

    #[test]
    fn round_trips_through_string() {
        for state in [
            RemoteUploadState::NotStarted,
            RemoteUploadState::Initialized,
            RemoteUploadState::Transferring,
            RemoteUploadState::Transferred,
            RemoteUploadState::RemoteProcessing,
            RemoteUploadState::RemoteSucceeded,
            RemoteUploadState::RemoteFailed,
            RemoteUploadState::RemoteUnknown,
        ] {
            assert_eq!(state.as_str().parse::<RemoteUploadState>().unwrap(), state);
        }
    }
}
