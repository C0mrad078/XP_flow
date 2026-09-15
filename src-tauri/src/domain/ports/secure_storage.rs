use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SecureStorageError {
    #[error("secure storage backend unavailable: {0}")]
    Unavailable(String),

    #[error("secure storage operation failed: {0}")]
    Backend(String),
}

/// Cross-platform secure credential storage.
///
/// Backed by the OS keychain (macOS Keychain, Windows Credential Manager,
/// Secret Service on Linux). Nothing sensitive — OAuth tokens included —
/// may ever be written to SQLite or to logs; this is the only sanctioned
/// place for that data. Phase 1 does not store real platform credentials,
/// but the interface is exercised so future OAuth work has a tested seam.
#[async_trait]
pub trait SecureStorage: Send + Sync {
    async fn set(&self, key: &str, value: &str) -> Result<(), SecureStorageError>;
    async fn get(&self, key: &str) -> Result<Option<String>, SecureStorageError>;
    async fn delete(&self, key: &str) -> Result<(), SecureStorageError>;
}
