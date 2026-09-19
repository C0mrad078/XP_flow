/**
 * Frontend mirror of the Rust domain types serialized across IPC
 * (src-tauri/src/domain/*). Field names and casing must match serde's
 * output exactly — see src-tauri/src/domain for the source of truth.
 */

export type UUID = string;
/** RFC 3339 UTC timestamp string, as produced by `DateTime<Utc>::to_rfc3339()`. */
export type ISODateTime = string;

export type Platform = "youtube" | "tiktok" | "kwai";

export const PLATFORMS: readonly Platform[] = ["youtube", "tiktok", "kwai"];

export const PLATFORM_LABELS: Record<Platform, string> = {
  youtube: "YouTube",
  tiktok: "TikTok",
  kwai: "Kwai",
};

export interface Workspace {
  id: UUID;
  name: string;
  /** IANA timezone identifier (e.g. "America/Sao_Paulo") — every schedule
   * slot and calendar view is interpreted in this zone, never the OS's. */
  timezone: string;
  created_at: ISODateTime;
  updated_at: ISODateTime;
}

export type PublicationStatus =
  | "imported"
  | "validating"
  | "ready"
  | "queued"
  | "scheduled"
  | "uploading"
  | "processing"
  | "published"
  | "failed"
  | "retry_wait"
  | "auth_required"
  | "rate_limited"
  | "blocked"
  | "paused"
  | "cancelled"
  | "archived"
  | "duplicate";

export interface Publication {
  id: UUID;
  workspace_id: UUID;
  video_id: UUID;
  channel_id: UUID;
  platform_account_id: UUID | null;
  platform: Platform;
  status: PublicationStatus;
  title: string;
  description: string | null;
  hashtags: string[];
  priority: import("./media").VideoPriority;
  locked: boolean;
  scheduled_at: ISODateTime | null;
  published_at: ISODateTime | null;
  remote_id: string | null;
  retry_count: number;
  last_error: string | null;
  /** Stable `PublishError::code()` behind `last_error`, when it came from
   * a typed error — check this (never `last_error`'s prose) to detect
   * "UNKNOWN_REMOTE_RESULT" and refuse blind retry for it. */
  last_error_code: string | null;
  execution_key?: UUID | null;
  repost_of_publication_id?: UUID | null;
  reconciliation_result?: string | null;
  reconciled_at?: ISODateTime | null;
  created_at: ISODateTime;
  updated_at: ISODateTime;
}

/** The one error code that must never be offered a plain "Retry" action
 * — an ambiguous remote outcome that was never resolved automatically to
 * protect against creating a duplicate post (see docs/publishing-ui.md). */
export const UNKNOWN_REMOTE_RESULT_CODE = "UNKNOWN_REMOTE_RESULT";

/** Derived (never persisted) — a Scheduled publication whose scheduled_at
 * has already passed. Mirrors `Publication::is_overdue` in Rust. */
export function isPublicationOverdue(publication: Publication, now: Date = new Date()): boolean {
  return (
    publication.status === "scheduled" &&
    publication.scheduled_at !== null &&
    new Date(publication.scheduled_at) < now
  );
}

export type ActivityCategory = "system" | "content" | "publication" | "platform" | "warning" | "error";
export type ActivityLevel = "info" | "success" | "warning" | "error";

export interface ActivityEvent {
  id: UUID;
  workspace_id: UUID | null;
  category: ActivityCategory;
  level: ActivityLevel;
  message: string;
  metadata_json: string | null;
  created_at: ISODateTime;
}

export type NotificationType = "info" | "success" | "warning" | "error";

export interface AppNotification {
  id: UUID;
  workspace_id: UUID | null;
  notification_type: NotificationType;
  title: string;
  message: string;
  read: boolean;
  created_at: ISODateTime;
}

export type ThemePreference = "dark" | "light" | "system";

export interface AppSettings {
  theme: ThemePreference;
  launch_on_startup: boolean;
}

export interface MediaToolStatus {
  available: boolean;
  path: string | null;
  version: string | null;
}

export interface MediaToolchainStatus {
  ffmpeg: MediaToolStatus;
  ffprobe: MediaToolStatus;
}

export interface AppInfo {
  version: string;
  os: string;
  arch: string;
  data_dir: string;
  log_dir: string;
  database_path: string;
}

/** Mirrors src-tauri/src/error.rs::ErrorCode. */
export type ErrorCode =
  "VALIDATION" | "DATABASE" | "AUTHENTICATION" | "NETWORK" | "RATE_LIMIT" | "MEDIA" | "PLATFORM" | "INTERNAL";

/** Mirrors src-tauri/src/error.rs::AppError. */
export interface AppError {
  code: ErrorCode;
  user_message: string;
  developer_message: string;
}

export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    "user_message" in value &&
    "developer_message" in value
  );
}
