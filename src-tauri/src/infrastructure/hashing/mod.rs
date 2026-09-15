//! Exact (SHA-256) and near-duplicate (perceptual dHash) content hashing
//! — sections 26-28 of the Phase 2 brief.

pub mod content_hash;
pub mod perceptual_hash;

pub use content_hash::Sha256ContentHashService;
pub use perceptual_hash::DHashPerceptualHashService;
