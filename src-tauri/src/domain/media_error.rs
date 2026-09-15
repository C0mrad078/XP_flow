use thiserror::Error;

/// Structured media-pipeline errors (section 61). Every variant has a safe,
/// specific user-facing message; the underlying technical detail (a raw
/// ffprobe stderr dump, an `io::Error`, ...) is carried separately and only
/// ever logged, never serialized to the frontend as-is.
#[derive(Debug, Error)]
pub enum MediaError {
    #[error("file not found: {path}")]
    FileNotFound { path: String },

    #[error("file could not be read: {path} ({detail})")]
    Unreadable { path: String, detail: String },

    #[error("ffprobe failed to analyze {path}: {detail}")]
    FfprobeFailed { path: String, detail: String },

    #[error("unsupported format: {extension}")]
    UnsupportedFormat { extension: String },

    #[error("video appears corrupted: {path}")]
    CorruptedVideo { path: String },

    #[error("thumbnail generation failed for {path}: {detail}")]
    ThumbnailFailed { path: String, detail: String },

    #[error("permission denied accessing source: {path}")]
    SourcePermissionDenied { path: String },
}

impl MediaError {
    pub fn code(&self) -> &'static str {
        match self {
            MediaError::FileNotFound { .. } => "MEDIA_FILE_NOT_FOUND",
            MediaError::Unreadable { .. } => "MEDIA_UNREADABLE",
            MediaError::FfprobeFailed { .. } => "FFPROBE_FAILED",
            MediaError::UnsupportedFormat { .. } => "UNSUPPORTED_FORMAT",
            MediaError::CorruptedVideo { .. } => "CORRUPTED_VIDEO",
            MediaError::ThumbnailFailed { .. } => "THUMBNAIL_FAILED",
            MediaError::SourcePermissionDenied { .. } => "SOURCE_PERMISSION_DENIED",
        }
    }

    /// Safe to show a user directly (section 61).
    pub fn user_message(&self) -> String {
        match self {
            MediaError::FileNotFound { .. } => "That file could not be found.".to_string(),
            MediaError::Unreadable { .. } => "That file could not be read.".to_string(),
            MediaError::FfprobeFailed { .. } => {
                "XP FLOW couldn't analyze this video file.".to_string()
            }
            MediaError::UnsupportedFormat { extension } => {
                format!("\"{extension}\" files aren't supported yet.")
            }
            MediaError::CorruptedVideo { .. } => {
                "This video file appears to be corrupted.".to_string()
            }
            MediaError::ThumbnailFailed { .. } => {
                "A thumbnail could not be generated for this video.".to_string()
            }
            MediaError::SourcePermissionDenied { .. } => {
                "XP FLOW doesn't have permission to access this folder.".to_string()
            }
        }
    }
}
