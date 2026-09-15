use std::path::Path;

use async_trait::async_trait;
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;

use crate::domain::media_error::MediaError;
use crate::domain::ports::hashing::ContentHashService;

const CHUNK_SIZE: usize = 1024 * 1024; // 1 MiB — never load a whole file into memory (section 27).

pub struct Sha256ContentHashService;

impl Sha256ContentHashService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Sha256ContentHashService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContentHashService for Sha256ContentHashService {
    async fn hash_file(&self, path: &Path) -> Result<String, MediaError> {
        let mut file = tokio::fs::File::open(path)
            .await
            .map_err(|e| MediaError::Unreadable {
                path: path.display().to_string(),
                detail: e.to_string(),
            })?;

        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; CHUNK_SIZE];

        loop {
            let bytes_read = file
                .read(&mut buffer)
                .await
                .map_err(|e| MediaError::Unreadable {
                    path: path.display().to_string(),
                    detail: e.to_string(),
                })?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(format!("{:x}", hasher.finalize()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hashes_a_file_streaming() {
        let dir = std::env::temp_dir().join(format!("xpflow-hash-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("sample.bin");
        std::fs::write(&path, b"hello xp flow").unwrap();

        let service = Sha256ContentHashService::new();
        let hash = service.hash_file(&path).await.unwrap();

        assert_eq!(
            hash,
            "9181b36f73f93d2424479840b60f8cf4f762aceee9a777d13069f4943fc963e0"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
