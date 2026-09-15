use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rand::RngCore;

/// AES-256-GCM encryption at rest for every persisted token (section 84).
/// The 12-byte nonce is generated fresh per call and stored alongside the
/// ciphertext (`nonce || ciphertext`, base64-encoded as one string) —
/// standard AEAD practice; the nonce is not secret, only unique-per-key.
#[derive(Clone)]
pub struct Cipher {
    key: [u8; 32],
}

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("encryption failed")]
    EncryptFailed,
    #[error(
        "decryption failed — the ciphertext may be corrupted or the master key may have changed"
    )]
    DecryptFailed,
    #[error("malformed ciphertext encoding")]
    MalformedCiphertext,
}

impl Cipher {
    pub fn new(master_key: [u8; 32]) -> Self {
        Self { key: master_key }
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<String, CryptoError> {
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|_| CryptoError::EncryptFailed)?;

        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);
        Ok(STANDARD.encode(combined))
    }

    pub fn decrypt(&self, encoded: &str) -> Result<String, CryptoError> {
        let combined = STANDARD
            .decode(encoded)
            .map_err(|_| CryptoError::MalformedCiphertext)?;
        if combined.len() < 12 {
            return Err(CryptoError::MalformedCiphertext);
        }
        let (nonce_bytes, ciphertext) = combined.split_at(12);
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&self.key));
        let plaintext = cipher
            .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
            .map_err(|_| CryptoError::DecryptFailed)?;
        String::from_utf8(plaintext).map_err(|_| CryptoError::DecryptFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cipher() -> Cipher {
        Cipher::new([42u8; 32])
    }

    #[test]
    fn round_trips_a_token() {
        let cipher = cipher();
        let encrypted = cipher.encrypt("super-secret-refresh-token").unwrap();
        assert_eq!(
            cipher.decrypt(&encrypted).unwrap(),
            "super-secret-refresh-token"
        );
    }

    #[test]
    fn ciphertext_never_contains_the_plaintext() {
        let cipher = cipher();
        let encrypted = cipher.encrypt("super-secret-refresh-token").unwrap();
        assert!(!encrypted.contains("super-secret-refresh-token"));
    }

    #[test]
    fn two_encryptions_of_the_same_plaintext_differ() {
        let cipher = cipher();
        let a = cipher.encrypt("same-value").unwrap();
        let b = cipher.encrypt("same-value").unwrap();
        assert_ne!(a, b, "nonce must be fresh per encryption");
    }

    #[test]
    fn decrypting_with_the_wrong_key_fails_instead_of_returning_garbage() {
        let encrypted = Cipher::new([1u8; 32]).encrypt("value").unwrap();
        assert!(Cipher::new([2u8; 32]).decrypt(&encrypted).is_err());
    }

    #[test]
    fn decrypting_malformed_input_fails_cleanly() {
        assert!(cipher().decrypt("not-valid-ciphertext").is_err());
    }
}
