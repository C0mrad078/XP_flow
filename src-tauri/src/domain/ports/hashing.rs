use std::path::Path;

use async_trait::async_trait;

use crate::domain::media_error::MediaError;

/// Streamed SHA-256 of a file's bytes (section 26/27) — exact-duplicate
/// detection. Implementations must never read the whole file into memory
/// at once.
#[async_trait]
pub trait ContentHashService: Send + Sync {
    async fn hash_file(&self, path: &Path) -> Result<String, MediaError>;
}

/// A lightweight difference-hash (dHash) computed from a video's
/// thumbnail, used for near-duplicate detection (section 28). This is
/// explicitly *not* full video perceptual hashing — one frame's
/// fingerprint, nothing more — and every match derived from it is stored
/// with a similarity score, never treated as certain (section 28: "do not
/// pretend near-duplicate detection is perfect").
#[async_trait]
pub trait PerceptualHashService: Send + Sync {
    async fn hash_image(&self, image_path: &Path) -> Result<String, MediaError>;

    /// Hamming distance between two hex-encoded hashes produced by
    /// `hash_image`, normalized to a 0.0..=1.0 similarity score.
    fn similarity(&self, hash_a: &str, hash_b: &str) -> f64;
}
