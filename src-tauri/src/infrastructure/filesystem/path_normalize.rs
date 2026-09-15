use std::path::{Path, PathBuf};

/// Centralizes filesystem path handling (section 63) — nothing else in the
/// codebase should manually concatenate paths with a hardcoded `/` or `\`
/// separator. `PathBuf::join` already does the right thing per-OS; this
/// module exists so the *canonicalization* and *supported-extension*
/// checks (which need to be consistent everywhere) have one home.
pub fn canonicalize_best_effort(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub const SUPPORTED_VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "mkv", "webm"];

pub fn is_supported_video_extension(path: &Path) -> bool {
    extension_lowercase(path).is_some_and(|ext| SUPPORTED_VIDEO_EXTENSIONS.contains(&ext.as_str()))
}

/// Lowercased extension without the leading dot — handles mixed-case
/// extensions from Windows/network shares (`.MP4`, `.Mp4`, ...) uniformly.
pub fn extension_lowercase(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_supported_extensions_case_insensitively() {
        assert!(is_supported_video_extension(Path::new("clip.mp4")));
        assert!(is_supported_video_extension(Path::new("clip.MP4")));
        assert!(is_supported_video_extension(Path::new("clip.MOV")));
        assert!(is_supported_video_extension(Path::new("clip.webm")));
        assert!(!is_supported_video_extension(Path::new("clip.avi")));
        assert!(!is_supported_video_extension(Path::new("clip")));
    }

    #[test]
    fn handles_spaces_and_unicode_filenames() {
        assert!(is_supported_video_extension(Path::new(
            "Neymar fala sobre Messi.mp4"
        )));
        assert!(is_supported_video_extension(Path::new(
            "Nível 1 — clipe.mov"
        )));
    }
}
