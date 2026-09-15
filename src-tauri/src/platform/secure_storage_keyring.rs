use async_trait::async_trait;

use crate::domain::ports::secure_storage::{SecureStorage, SecureStorageError};

const SERVICE_NAME: &str = "com.xpflow.app";

/// OS-native secure storage: Keychain on macOS, Credential Manager on
/// Windows, Secret Service on Linux — selected at compile time by the
/// `keyring` crate's platform features. `keyring::Entry` calls are
/// synchronous/blocking, so they run on Tokio's blocking pool to avoid
/// stalling the async runtime (and, by extension, the UI).
pub struct KeyringSecureStorage;

impl KeyringSecureStorage {
    pub fn new() -> Self {
        Self
    }

    fn entry(key: &str) -> Result<keyring::Entry, SecureStorageError> {
        keyring::Entry::new(SERVICE_NAME, key)
            .map_err(|e| SecureStorageError::Unavailable(e.to_string()))
    }
}

impl Default for KeyringSecureStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecureStorage for KeyringSecureStorage {
    async fn set(&self, key: &str, value: &str) -> Result<(), SecureStorageError> {
        let key = key.to_owned();
        let value = value.to_owned();
        tauri::async_runtime::spawn_blocking(move || {
            let entry = Self::entry(&key)?;
            entry
                .set_password(&value)
                .map_err(|e| SecureStorageError::Backend(e.to_string()))
        })
        .await
        .map_err(|e| SecureStorageError::Backend(e.to_string()))?
    }

    async fn get(&self, key: &str) -> Result<Option<String>, SecureStorageError> {
        let key = key.to_owned();
        tauri::async_runtime::spawn_blocking(move || {
            let entry = Self::entry(&key)?;
            match entry.get_password() {
                Ok(value) => Ok(Some(value)),
                Err(keyring::Error::NoEntry) => Ok(None),
                Err(e) => Err(SecureStorageError::Backend(e.to_string())),
            }
        })
        .await
        .map_err(|e| SecureStorageError::Backend(e.to_string()))?
    }

    async fn delete(&self, key: &str) -> Result<(), SecureStorageError> {
        let key = key.to_owned();
        tauri::async_runtime::spawn_blocking(move || {
            let entry = Self::entry(&key)?;
            match entry.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
                Err(e) => Err(SecureStorageError::Backend(e.to_string())),
            }
        })
        .await
        .map_err(|e| SecureStorageError::Backend(e.to_string()))?
    }
}
