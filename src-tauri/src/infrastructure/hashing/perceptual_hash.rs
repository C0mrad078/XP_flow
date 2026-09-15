use std::path::Path;

use async_trait::async_trait;
use image::imageops::FilterType;

use crate::domain::media_error::MediaError;
use crate::domain::ports::hashing::PerceptualHashService;

/// A single-frame difference-hash (dHash), computed from a video's
/// generated thumbnail (section 28). This is deliberately not full-video
/// perceptual hashing — it is a lightweight, honestly-scoped fingerprint:
/// resize to 9x8 grayscale, compare each pixel to its right neighbor, pack
/// the 64 comparison bits into a hex string. Two videos whose thumbnails
/// hash to a small Hamming distance are *probably* the same or very
/// similar content — never treated as certain (see `similarity`).
pub struct DHashPerceptualHashService;

impl DHashPerceptualHashService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DHashPerceptualHashService {
    fn default() -> Self {
        Self::new()
    }
}

const HASH_WIDTH: u32 = 9;
const HASH_HEIGHT: u32 = 8;

#[async_trait]
impl PerceptualHashService for DHashPerceptualHashService {
    async fn hash_image(&self, image_path: &Path) -> Result<String, MediaError> {
        let path = image_path.to_path_buf();
        tokio::task::spawn_blocking(move || compute_dhash(&path))
            .await
            .map_err(|e| MediaError::ThumbnailFailed {
                path: image_path.display().to_string(),
                detail: format!("perceptual hash task panicked: {e}"),
            })?
    }

    fn similarity(&self, hash_a: &str, hash_b: &str) -> f64 {
        let (Ok(a), Ok(b)) = (
            u64::from_str_radix(hash_a, 16),
            u64::from_str_radix(hash_b, 16),
        ) else {
            return 0.0;
        };
        let distance = (a ^ b).count_ones();
        1.0 - (distance as f64 / 64.0)
    }
}

fn compute_dhash(path: &Path) -> Result<String, MediaError> {
    let img = image::open(path).map_err(|e| MediaError::ThumbnailFailed {
        path: path.display().to_string(),
        detail: e.to_string(),
    })?;

    let small = img
        .resize_exact(HASH_WIDTH, HASH_HEIGHT, FilterType::Triangle)
        .to_luma8();

    let mut bits: u64 = 0;
    let mut bit_index = 0;
    for y in 0..HASH_HEIGHT {
        for x in 0..HASH_WIDTH - 1 {
            let left = small.get_pixel(x, y).0[0];
            let right = small.get_pixel(x + 1, y).0[0];
            if left > right {
                bits |= 1 << bit_index;
            }
            bit_index += 1;
        }
    }

    Ok(format!("{bits:016x}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_hashes_are_fully_similar() {
        let service = DHashPerceptualHashService::new();
        assert_eq!(
            service.similarity("abcd1234abcd1234", "abcd1234abcd1234"),
            1.0
        );
    }

    #[test]
    fn maximally_different_hashes_have_zero_similarity() {
        let service = DHashPerceptualHashService::new();
        assert_eq!(
            service.similarity("0000000000000000", "ffffffffffffffff"),
            0.0
        );
    }

    #[test]
    fn invalid_hashes_yield_zero_similarity_instead_of_panicking() {
        let service = DHashPerceptualHashService::new();
        assert_eq!(service.similarity("not-hex", "abcd1234abcd1234"), 0.0);
    }
}
