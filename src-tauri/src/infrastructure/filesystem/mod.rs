//! Filesystem primitives shared across the ingestion pipeline: file
//! stability detection (section 14), path normalization (section 63),
//! reveal-in-file-manager (section 45) and cache accounting (section 67).

pub mod cache;
pub mod path_normalize;
pub mod reveal;
pub mod stability;

pub use cache::{clear_dir_contents, dir_size_bytes};
pub use path_normalize::{
    extension_lowercase, is_supported_video_extension, SUPPORTED_VIDEO_EXTENSIONS,
};
pub use reveal::reveal_in_file_manager;
pub use stability::{FileStabilityChecker, StabilityConfig};
