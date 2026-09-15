use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};
use tokio::sync::mpsc;
use tracing::{error, warn};
use uuid::Uuid;

use crate::domain::media_error::MediaError;
use crate::infrastructure::filesystem::is_supported_video_extension;

/// A file was created or finished being written to inside a watched root.
/// The watcher does *not* run stability checks or ingestion itself — it
/// only resolves "which source does this path belong to" and hands the
/// raw event to whoever is listening (the job runner). This keeps the
/// watcher a thin, restartable OS-event adapter (section 15).
#[derive(Debug, Clone)]
pub struct WatchEvent {
    pub source_id: Uuid,
    pub path: PathBuf,
}

struct WatchedRoot {
    source_id: Uuid,
}

/// Cross-platform folder watcher (section 15) built on `notify` +
/// `notify-debouncer-full`. Multiple roots can be watched at once; each is
/// tagged with the [`Uuid`] of the [`crate::domain::video_source::VideoSource`]
/// it belongs to, resolved by longest-prefix match when an event fires.
///
/// Debouncing is handled entirely by `notify-debouncer-full` (section 15:
/// "do not assume 1 OS event = 1 new file") — the underlying library
/// coalesces the burst of Create/Modify events a single file write
/// typically produces into one logical event per path.
pub struct FolderWatcherService {
    debouncer: Mutex<Option<Debouncer<notify::RecommendedWatcher, RecommendedCache>>>,
    roots: std::sync::Arc<Mutex<HashMap<PathBuf, WatchedRoot>>>,
}

impl FolderWatcherService {
    /// Starts the watcher. Returns the service plus a channel the caller
    /// reads discovered files from.
    pub fn start() -> (Self, mpsc::UnboundedReceiver<WatchEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let roots: std::sync::Arc<Mutex<HashMap<PathBuf, WatchedRoot>>> =
            std::sync::Arc::new(Mutex::new(HashMap::new()));
        let roots_for_callback = roots.clone();

        let handler = move |result: DebounceEventResult| {
            let events = match result {
                Ok(events) => events,
                Err(errors) => {
                    for e in errors {
                        error!(error = %e, "folder watcher error");
                    }
                    return;
                }
            };

            for event in events {
                if !matches!(
                    event.kind,
                    notify::EventKind::Create(_) | notify::EventKind::Modify(_)
                ) {
                    continue;
                }

                for path in &event.paths {
                    if !path.is_file() || !is_supported_video_extension(path) {
                        continue;
                    }

                    let Some(source_id) = resolve_source(&roots_for_callback, path) else {
                        continue;
                    };

                    if tx
                        .send(WatchEvent {
                            source_id,
                            path: path.clone(),
                        })
                        .is_err()
                    {
                        warn!("folder watcher event channel closed; dropping event");
                    }
                }
            }
        };

        let debouncer = new_debouncer(Duration::from_secs(2), None, handler)
            .map_err(|e| error!(error = %e, "failed to start folder watcher"))
            .ok();

        (
            Self {
                debouncer: Mutex::new(debouncer),
                roots,
            },
            rx,
        )
    }

    pub fn watch_root(
        &self,
        source_id: Uuid,
        path: &Path,
        recursive: bool,
    ) -> Result<(), MediaError> {
        let mode = if recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        let mut guard = self.debouncer.lock().unwrap();
        let Some(debouncer) = guard.as_mut() else {
            return Err(MediaError::SourcePermissionDenied {
                path: path.display().to_string(),
            });
        };

        debouncer
            .watch(path, mode)
            .map_err(|e| MediaError::SourcePermissionDenied {
                path: format!("{} ({e})", path.display()),
            })?;

        self.roots
            .lock()
            .unwrap()
            .insert(path.to_path_buf(), WatchedRoot { source_id });
        Ok(())
    }

    pub fn unwatch_root(&self, path: &Path) {
        if let Some(debouncer) = self.debouncer.lock().unwrap().as_mut() {
            let _ = debouncer.unwatch(path);
        }
        self.roots.lock().unwrap().remove(path);
    }

    /// Graceful shutdown (section 86): stops accepting new filesystem
    /// events. Dropping the debouncer stops its background thread.
    pub fn shutdown(&self) {
        *self.debouncer.lock().unwrap() = None;
        self.roots.lock().unwrap().clear();
    }
}

fn resolve_source(
    roots: &Mutex<HashMap<PathBuf, WatchedRoot>>,
    changed_path: &Path,
) -> Option<Uuid> {
    let roots = roots.lock().unwrap();
    roots
        .iter()
        .filter(|(root, _)| changed_path.starts_with(root))
        .max_by_key(|(root, _)| root.as_os_str().len())
        .map(|(_, watched)| watched.source_id)
}
