use std::path::Path;
use std::time::Duration;

use crate::domain::media_error::MediaError;

/// How the file-stability check (section 14) is tuned. A file appearing in
/// a watched export folder does not mean the export finished — Cut.pro (or
/// any tool) may still be writing to it. XP FLOW never analyzes a file
/// until its size has stopped changing across `required_stable_reads`
/// consecutive polls.
#[derive(Debug, Clone, Copy)]
pub struct StabilityConfig {
    pub poll_interval: Duration,
    pub required_stable_reads: u32,
    pub max_attempts: u32,
}

impl Default for StabilityConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_millis(500),
            required_stable_reads: 3,
            max_attempts: 40, // ~20s ceiling before giving up on a stuck/huge write
        }
    }
}

/// `DISCOVERED -> WAITING_FOR_FILE -> STABLE` (section 14). `INGESTING` is
/// the caller's responsibility once this returns `Ok`.
pub struct FileStabilityChecker {
    config: StabilityConfig,
}

impl FileStabilityChecker {
    pub fn new(config: StabilityConfig) -> Self {
        Self { config }
    }
}

impl Default for FileStabilityChecker {
    fn default() -> Self {
        Self::new(StabilityConfig::default())
    }
}

impl FileStabilityChecker {
    pub async fn wait_until_stable(&self, path: &Path) -> Result<(), MediaError> {
        let mut last_size: Option<u64> = None;
        let mut stable_reads = 0u32;

        for _ in 0..self.config.max_attempts {
            let metadata =
                tokio::fs::metadata(path)
                    .await
                    .map_err(|e| MediaError::FileNotFound {
                        path: format!("{} ({e})", path.display()),
                    })?;
            let size = metadata.len();

            if Some(size) == last_size {
                stable_reads += 1;
                if stable_reads >= self.config.required_stable_reads {
                    return self.ensure_openable(path).await;
                }
            } else {
                stable_reads = 0;
            }

            last_size = Some(size);
            tokio::time::sleep(self.config.poll_interval).await;
        }

        Err(MediaError::Unreadable {
            path: path.display().to_string(),
            detail: "file never stopped changing size — export may still be in progress"
                .to_string(),
        })
    }

    async fn ensure_openable(&self, path: &Path) -> Result<(), MediaError> {
        tokio::fs::File::open(path)
            .await
            .map(|_| ())
            .map_err(|e| MediaError::Unreadable {
                path: path.display().to_string(),
                detail: e.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fast_config() -> StabilityConfig {
        StabilityConfig {
            poll_interval: Duration::from_millis(5),
            required_stable_reads: 2,
            max_attempts: 20,
        }
    }

    #[tokio::test]
    async fn a_file_that_stops_growing_becomes_stable() {
        let dir = std::env::temp_dir().join(format!("xpflow-stability-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("clip.mp4");
        std::fs::write(&path, b"fixed content").unwrap();

        let checker = FileStabilityChecker::new(fast_config());
        checker
            .wait_until_stable(&path)
            .await
            .expect("a static file should stabilize");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_growing_file_is_not_treated_as_stable() {
        let dir = std::env::temp_dir().join(format!("xpflow-stability-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("clip.mp4");
        std::fs::write(&path, b"a").unwrap();

        let path_clone = path.clone();
        let writer = tokio::spawn(async move {
            for i in 0..30u8 {
                tokio::time::sleep(Duration::from_millis(5)).await;
                let _ = tokio::fs::write(&path_clone, vec![b'a'; 10 + i as usize]).await;
            }
        });

        let checker = FileStabilityChecker::new(StabilityConfig {
            poll_interval: Duration::from_millis(5),
            required_stable_reads: 2,
            max_attempts: 8, // give up well before the writer finishes
        });
        let result = checker.wait_until_stable(&path).await;
        assert!(
            result.is_err(),
            "a still-growing file must not be reported stable"
        );

        writer.abort();
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn missing_file_is_reported_as_not_found() {
        let checker = FileStabilityChecker::new(fast_config());
        let result = checker
            .wait_until_stable(Path::new("/nonexistent/xpflow-test-file.mp4"))
            .await;
        assert!(matches!(result, Err(MediaError::FileNotFound { .. })));
    }
}
