use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// Sums file sizes under `dir` (section 67 — Settings → Storage cache
/// sizes). Runs the walk on a blocking thread since it's plain
/// synchronous I/O over a potentially large directory.
pub async fn dir_size_bytes(dir: PathBuf) -> u64 {
    tokio::task::spawn_blocking(move || {
        WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum()
    })
    .await
    .unwrap_or(0)
}

/// Deletes every file directly under `dir` without removing the directory
/// itself — used for "Clear temporary cache" (section 67), which must
/// never touch `cache/thumbnails/` (only `cache/temp/`).
pub async fn clear_dir_contents(dir: PathBuf) -> std::io::Result<u64> {
    tokio::task::spawn_blocking(move || clear_dir_contents_sync(&dir))
        .await
        .unwrap_or(Ok(0))
}

fn clear_dir_contents_sync(dir: &Path) -> std::io::Result<u64> {
    let mut removed = 0u64;
    for entry in std::fs::read_dir(dir)?.filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
        removed += 1;
    }
    Ok(removed)
}
