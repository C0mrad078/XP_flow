use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use serde_json::json;
use tokio::sync::{mpsc, Semaphore};
use tracing::{info, warn};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::application::activity_service::ActivityService;
use crate::application::media_ingestion_service::{IngestOutcome, MediaIngestionService};
use crate::domain::activity_event::{ActivityCategory, ActivityLevel};
use crate::domain::errors::DomainError;
use crate::domain::ports::repositories::{VideoRepository, VideoSourceRepository};
use crate::domain::video_source::VideoSource;
use crate::domain::video_status::AvailabilityStatus;
use crate::infrastructure::filesystem::{is_supported_video_extension, FileStabilityChecker};
use crate::infrastructure::watcher::{FolderWatcherService, WatchEvent};
use crate::jobs::{Job, JobRepository, JobType};
use crate::services::notification_service::NotificationService;

/// How many files can be probed/hashed/thumbnailed at once (section 56/87
/// — "2-4 media analysis workers", never hundreds of concurrent FFprobe
/// processes).
const MAX_CONCURRENT_INGESTIONS: usize = 3;

#[derive(Debug, Serialize)]
pub struct RejectedImport {
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Default)]
pub struct ImportSummary {
    pub imported: usize,
    pub duplicates: usize,
    pub moved: usize,
    pub already_indexed: usize,
    pub rejected: Vec<RejectedImport>,
}

#[derive(Debug, Serialize, Default)]
pub struct ReconcileSummary {
    pub scanned: usize,
    pub imported: usize,
    pub already_indexed: usize,
    pub marked_missing: usize,
    pub rejected: usize,
}

/// Coordinates every way a file enters XP FLOW (section 13's
/// `MediaIngestionService` unifies *what* ingestion does; this is *when*
/// and *how many at once*): manual/drag-and-drop import, folder-watcher
/// dispatch, and folder reconciliation (startup + periodic, sections
/// 16/17), all funneled through one bounded-concurrency semaphore and the
/// `jobs` table for idempotency (section 58) and crash recovery (85/86).
pub struct JobRunner {
    job_repo: Arc<dyn JobRepository>,
    video_repo: Arc<dyn VideoRepository>,
    source_repo: Arc<dyn VideoSourceRepository>,
    ingestion: Arc<MediaIngestionService>,
    activity_service: Arc<ActivityService>,
    notification_service: Arc<NotificationService>,
    stability_checker: Arc<FileStabilityChecker>,
    watcher: Arc<FolderWatcherService>,
    semaphore: Arc<Semaphore>,
}

impl JobRunner {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        job_repo: Arc<dyn JobRepository>,
        video_repo: Arc<dyn VideoRepository>,
        source_repo: Arc<dyn VideoSourceRepository>,
        ingestion: Arc<MediaIngestionService>,
        activity_service: Arc<ActivityService>,
        notification_service: Arc<NotificationService>,
        watcher: Arc<FolderWatcherService>,
    ) -> Self {
        Self {
            job_repo,
            video_repo,
            source_repo,
            ingestion,
            activity_service,
            notification_service,
            stability_checker: Arc::new(FileStabilityChecker::default()),
            watcher,
            semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_INGESTIONS)),
        }
    }

    // ---------------------------------------------------------------
    // Startup / shutdown
    // ---------------------------------------------------------------

    /// Section 84/85/86: any job still `running` belonged to a process
    /// that was killed — it can never complete, so it's reset to `failed`
    /// rather than blocking its dedupe key forever.
    pub async fn recover_orphaned_jobs(&self) -> u64 {
        match self
            .job_repo
            .fail_orphaned_running_jobs("interrupted by shutdown")
            .await
        {
            Ok(count) => count,
            Err(err) => {
                warn!(error = %err, "failed to recover orphaned jobs");
                0
            }
        }
    }

    /// Registers watchers and kicks off a reconciliation scan for every
    /// enabled, folder-backed source (section 16).
    pub async fn startup_reconciliation(self: &Arc<Self>, workspace_id: Uuid) {
        let sources = match self.source_repo.list_for_workspace(workspace_id).await {
            Ok(sources) => sources,
            Err(err) => {
                warn!(error = %err, "failed to list sources for startup reconciliation");
                return;
            }
        };

        for source in sources {
            if !source.enabled || !source.is_folder_backed() {
                continue;
            }

            if source.watch_enabled {
                self.start_source_watch(&source);
            }

            let this = self.clone();
            let source_id = source.id;
            tokio::spawn(async move {
                let _ = this.reconcile_source(source_id).await;
            });
        }
    }

    pub fn start_source_watch(&self, source: &VideoSource) {
        let Some(folder_path) = &source.folder_path else {
            return;
        };
        if let Err(err) =
            self.watcher
                .watch_root(source.id, Path::new(folder_path), source.recursive)
        {
            warn!(error = %err, source = %source.name, "failed to start watching source");
        } else {
            info!(source = %source.name, path = %folder_path, "watching content source");
        }
    }

    pub fn stop_source_watch(&self, source: &VideoSource) {
        if let Some(folder_path) = &source.folder_path {
            self.watcher.unwatch_root(Path::new(folder_path));
        }
    }

    /// Section 15/86: stop accepting new filesystem events as part of a
    /// clean app shutdown.
    pub fn shutdown(&self) {
        self.watcher.shutdown();
    }

    /// Section 17: a lightweight periodic re-scan, as a backstop against
    /// filesystem events the OS watcher might have missed. Runs forever on
    /// its own task — call once at startup.
    pub fn spawn_periodic_reconciliation(self: Arc<Self>, workspace_id: Uuid, interval: Duration) {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.tick().await; // skip the immediate first tick — startup_reconciliation already covers it
            loop {
                ticker.tick().await;
                let Ok(sources) = self.source_repo.list_for_workspace(workspace_id).await else {
                    continue;
                };
                for source in sources
                    .into_iter()
                    .filter(|s| s.enabled && s.is_folder_backed())
                {
                    let _ = self.reconcile_source(source.id).await;
                }
            }
        });
    }

    // ---------------------------------------------------------------
    // Folder watcher dispatch
    // ---------------------------------------------------------------

    /// Reads watcher events forever, spawning a bounded-concurrency
    /// ingestion task per file. Call once at startup on the receiver
    /// returned by `FolderWatcherService::start`.
    pub fn spawn_watch_dispatch_loop(self: Arc<Self>, mut rx: mpsc::UnboundedReceiver<WatchEvent>) {
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let this = self.clone();
                tokio::spawn(async move {
                    this.handle_watch_event(event).await;
                });
            }
        });
    }

    async fn handle_watch_event(&self, event: WatchEvent) {
        let Ok(Some(source)) = self.source_repo.get(event.source_id).await else {
            return;
        };

        let dedupe_key = format!("ingest:{}", event.path.display());
        let job = Job::new(
            JobType::IngestVideo,
            json!({ "path": event.path, "source_id": event.source_id }).to_string(),
            Some(dedupe_key),
        );
        let my_job_id = job.id;
        let enqueued = match self.job_repo.enqueue(&job).await {
            Ok(j) => j,
            Err(err) => {
                warn!(error = %err, "failed to enqueue ingest job");
                return;
            }
        };
        if enqueued.id != my_job_id {
            // Another in-flight job already owns this file — section 58.
            return;
        }

        let _permit = self.semaphore.acquire().await;
        let _ = self.job_repo.mark_running(my_job_id).await;

        let outcome = self
            .ingest_one(
                source.workspace_id,
                source.id,
                source.channel_id,
                &event.path,
            )
            .await;
        self.finish_job_for_outcome(my_job_id, &event.path, &outcome)
            .await;
    }

    // ---------------------------------------------------------------
    // Manual import / drag-and-drop (section 11/12/13)
    // ---------------------------------------------------------------

    pub async fn import_paths(
        &self,
        workspace_id: Uuid,
        source_id: Uuid,
        channel_override: Option<Uuid>,
        paths: Vec<PathBuf>,
    ) -> ImportSummary {
        let mut handles = Vec::with_capacity(paths.len());

        for path in paths {
            let semaphore = self.semaphore.clone();
            let stability_checker = self.stability_checker.clone();
            let ingestion = self.ingestion.clone();

            handles.push(tokio::spawn(async move {
                let _permit = semaphore.acquire().await;
                if let Err(err) = stability_checker.wait_until_stable(&path).await {
                    return (path, IngestOutcome::Rejected(err));
                }
                let outcome = ingestion
                    .ingest_path(workspace_id, source_id, channel_override, &path)
                    .await;
                (path, outcome)
            }));
        }

        let mut summary = ImportSummary::default();
        for handle in handles {
            let Ok((path, outcome)) = handle.await else {
                continue;
            };
            self.record_outcome(&mut summary, &path, outcome).await;
        }

        self.notify_import_summary(&summary).await;
        summary
    }

    async fn record_outcome(
        &self,
        summary: &mut ImportSummary,
        path: &Path,
        outcome: IngestOutcome,
    ) {
        match &outcome {
            IngestOutcome::Created(_) => summary.imported += 1,
            IngestOutcome::Duplicate { .. } => summary.duplicates += 1,
            IngestOutcome::Moved { .. } => summary.moved += 1,
            IngestOutcome::AlreadyIndexed { .. } => summary.already_indexed += 1,
            IngestOutcome::Rejected(err) => summary.rejected.push(RejectedImport {
                path: path.display().to_string(),
                reason: err.user_message(),
            }),
        }
        self.log_ingest_outcome(path, &outcome).await;
    }

    /// Activity trail (section 59) for a single file's ingestion result —
    /// shared by manual/drag-and-drop import, watcher dispatch and
    /// reconciliation so the log reads the same regardless of entry point.
    async fn log_ingest_outcome(&self, path: &Path, outcome: &IngestOutcome) {
        let (category, level, message) = match outcome {
            IngestOutcome::Created(video) => (
                ActivityCategory::Content,
                ActivityLevel::Success,
                format!("Video imported: {}", video.display_title),
            ),
            IngestOutcome::Duplicate { existing } => (
                ActivityCategory::Content,
                ActivityLevel::Info,
                format!(
                    "Duplicate detected: \"{}\" is already in XP FLOW",
                    existing.display_title
                ),
            ),
            IngestOutcome::Moved { updated } => (
                ActivityCategory::Content,
                ActivityLevel::Info,
                format!("Video moved: {}", updated.display_title),
            ),
            IngestOutcome::AlreadyIndexed { .. } => return,
            IngestOutcome::Rejected(err) => (
                ActivityCategory::Error,
                ActivityLevel::Error,
                format!(
                    "Failed to import {}: {}",
                    path.display(),
                    err.user_message()
                ),
            ),
        };
        let _ = self.activity_service.log(category, level, message).await;
    }

    /// Section 60: don't notify per-file during a large batch — one
    /// grouped notification for the whole import.
    async fn notify_import_summary(&self, summary: &ImportSummary) {
        if summary.imported == 0 && summary.rejected.is_empty() && summary.duplicates == 0 {
            return;
        }

        let mut parts = Vec::new();
        if summary.imported > 0 {
            parts.push(format!("{} imported", summary.imported));
        }
        if summary.duplicates > 0 {
            parts.push(format!(
                "{} duplicate{}",
                summary.duplicates,
                if summary.duplicates == 1 { "" } else { "s" }
            ));
        }
        if !summary.rejected.is_empty() {
            parts.push(format!("{} invalid", summary.rejected.len()));
        }

        let notification_type = if summary.rejected.is_empty() {
            crate::domain::notification::NotificationType::Success
        } else {
            crate::domain::notification::NotificationType::Warning
        };

        let _ = self
            .notification_service
            .notify(notification_type, "Import completed", parts.join(", "))
            .await;
    }

    async fn ingest_one(
        &self,
        workspace_id: Uuid,
        source_id: Uuid,
        channel_id: Option<Uuid>,
        path: &Path,
    ) -> IngestOutcome {
        if let Err(err) = self.stability_checker.wait_until_stable(path).await {
            return IngestOutcome::Rejected(err);
        }
        self.ingestion
            .ingest_path(workspace_id, source_id, channel_id, path)
            .await
    }

    async fn finish_job_for_outcome(&self, job_id: Uuid, path: &Path, outcome: &IngestOutcome) {
        match outcome {
            IngestOutcome::Rejected(err) => {
                let _ = self.job_repo.mark_failed(job_id, &err.to_string()).await;
            }
            _ => {
                let _ = self.job_repo.mark_succeeded(job_id).await;
            }
        }
        self.log_ingest_outcome(path, outcome).await;
    }

    // ---------------------------------------------------------------
    // Reconciliation (section 16/17/32/62)
    // ---------------------------------------------------------------

    pub async fn reconcile_source(&self, source_id: Uuid) -> Result<ReconcileSummary, DomainError> {
        let Some(source) = self.source_repo.get(source_id).await? else {
            return Err(DomainError::NotFound {
                entity: "VideoSource",
                id: source_id.to_string(),
            });
        };
        let Some(folder_path) = &source.folder_path else {
            return Ok(ReconcileSummary::default());
        };

        let _ = self
            .activity_service
            .log(
                ActivityCategory::System,
                ActivityLevel::Info,
                format!("Folder scan started: {}", source.name),
            )
            .await;

        let mut summary = ReconcileSummary::default();
        let root = PathBuf::from(folder_path);
        let max_depth = if source.recursive { usize::MAX } else { 1 };

        let mut seen_paths: HashSet<String> = HashSet::new();

        let entries: Vec<PathBuf> = WalkDir::new(&root)
            .max_depth(max_depth)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file() && is_supported_video_extension(e.path()))
            .map(|e| e.into_path())
            .collect();

        for path in entries {
            summary.scanned += 1;
            let path_string = path.to_string_lossy().to_string();
            seen_paths.insert(path_string.clone());

            match self
                .video_repo
                .get_by_path(source.workspace_id, &path_string)
                .await
            {
                Ok(Some(mut existing)) => {
                    summary.already_indexed += 1;
                    if existing.availability_status != AvailabilityStatus::Available {
                        existing.availability_status = AvailabilityStatus::Available;
                        existing.last_seen_at = chrono::Utc::now();
                        let _ = self.video_repo.update(&existing).await;
                    }
                    continue;
                }
                Ok(None) => {}
                Err(_) => continue,
            }

            let outcome = self
                .ingest_one(source.workspace_id, source.id, source.channel_id, &path)
                .await;
            match &outcome {
                IngestOutcome::Created(_) | IngestOutcome::Moved { .. } => summary.imported += 1,
                IngestOutcome::AlreadyIndexed { .. } => summary.already_indexed += 1,
                IngestOutcome::Duplicate { .. } => {}
                IngestOutcome::Rejected(_) => summary.rejected += 1,
            }
            self.log_ingest_outcome(&path, &outcome).await;
        }

        // Anything previously indexed for this source but not seen in this
        // scan, and no longer present on disk, is missing (section 62) —
        // never deleted, just flagged.
        if let Ok(known) = self.video_repo.list_paths_for_source(source.id).await {
            for (video_id, known_path) in known {
                if seen_paths.contains(&known_path) || Path::new(&known_path).is_file() {
                    continue;
                }
                if let Ok(Some(mut video)) = self.video_repo.get(video_id).await {
                    video.availability_status = AvailabilityStatus::Missing;
                    video.updated_at = chrono::Utc::now();
                    if self.video_repo.update(&video).await.is_ok() {
                        summary.marked_missing += 1;
                        let _ = self
                            .activity_service
                            .log(
                                ActivityCategory::Warning,
                                ActivityLevel::Warning,
                                format!("Video file missing: {}", video.display_title),
                            )
                            .await;
                    }
                }
            }
        }

        let _ = self
            .activity_service
            .log(
                ActivityCategory::System,
                ActivityLevel::Success,
                format!(
                    "Folder scan completed: {} ({} scanned, {} imported, {} missing)",
                    source.name, summary.scanned, summary.imported, summary.marked_missing
                ),
            )
            .await;

        // Reaching this point means the walk itself completed without a
        // fatal error (per-file failures are already reflected in
        // `summary.rejected` above), so the source's `last_error` clears.
        let mut updated_source = source.clone();
        updated_source.last_scan_at = Some(chrono::Utc::now());
        updated_source.last_error = None;
        updated_source.updated_at = chrono::Utc::now();
        let _ = self.source_repo.update(&updated_source).await;

        Ok(summary)
    }
}
