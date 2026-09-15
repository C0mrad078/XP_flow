use std::path::Path;

use crate::domain::media_error::MediaError;

/// Cross-platform "Show in Finder" / "Show in File Explorer" (section 45).
/// A thin wrapper over the `opener` crate so the rest of the codebase
/// never needs its own `cfg(target_os = ...)` branch for this.
pub fn reveal_in_file_manager(path: &Path) -> Result<(), MediaError> {
    opener::reveal(path).map_err(|e| MediaError::Unreadable {
        path: path.display().to_string(),
        detail: e.to_string(),
    })
}
