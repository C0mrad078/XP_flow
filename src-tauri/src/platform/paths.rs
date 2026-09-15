use std::path::PathBuf;

use directories::BaseDirs;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PathsError {
    #[error("could not resolve the operating system's application data directory")]
    NoBaseDirs,

    #[error("failed to create directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// XP FLOW's resolved on-disk locations. Every OS-specific detail (the
/// exact application-support/roaming-appdata directory) is isolated here —
/// nothing outside `platform` hardcodes a path (Rule 5).
///
/// Resulting locations:
/// - Windows: `%APPDATA%/XP FLOW/`
/// - macOS:   `~/Library/Application Support/XP FLOW/`
/// - Linux:   `~/.local/share/XP FLOW/` (development fallback)
///
/// Cache is organized per section 66 of the Phase 2 brief:
/// `cache/thumbnails/` (generated JPEGs, one per video, named by video id)
/// and `cache/temp/` (scratch space, safe to clear at any time). Neither
/// ever lives beside the user's original video files (section 65).
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data_dir: PathBuf,
    pub log_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub thumbnail_cache_dir: PathBuf,
    pub temp_cache_dir: PathBuf,
}

impl AppPaths {
    pub fn resolve() -> Result<Self, PathsError> {
        let base = BaseDirs::new().ok_or(PathsError::NoBaseDirs)?;
        let data_dir = base.data_dir().join("XP FLOW");
        let log_dir = data_dir.join("logs");
        let cache_dir = data_dir.join("cache");
        let thumbnail_cache_dir = cache_dir.join("thumbnails");
        let temp_cache_dir = cache_dir.join("temp");

        for dir in [
            &data_dir,
            &log_dir,
            &cache_dir,
            &thumbnail_cache_dir,
            &temp_cache_dir,
        ] {
            std::fs::create_dir_all(dir).map_err(|source| PathsError::CreateDir {
                path: dir.clone(),
                source,
            })?;
        }

        Ok(Self {
            data_dir,
            log_dir,
            cache_dir,
            thumbnail_cache_dir,
            temp_cache_dir,
        })
    }

    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join("xpflow.db")
    }

    pub fn thumbnail_path(&self, video_id: uuid::Uuid) -> PathBuf {
        self.thumbnail_cache_dir.join(format!("{video_id}.jpg"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_creates_directories() {
        let paths = AppPaths::resolve().expect("resolve should succeed on a dev machine");
        assert!(paths.data_dir.is_dir());
        assert!(paths.log_dir.is_dir());
        assert!(paths.cache_dir.is_dir());
        assert!(paths.thumbnail_cache_dir.is_dir());
        assert!(paths.temp_cache_dir.is_dir());
        assert!(paths.data_dir.ends_with("XP FLOW"));
    }
}
