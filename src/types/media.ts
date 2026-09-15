/**
 * Frontend mirror of the Rust Phase 2 media types serialized across IPC
 * (src-tauri/src/domain/video*.rs, application/*_service.rs). Field names
 * and casing must match serde's output exactly.
 */
import type { ISODateTime, UUID } from "./domain";

export type ValidationStatus =
  "pending" | "validating" | "valid" | "invalid" | "unsupported" | "corrupted" | "missing";

export const VALIDATION_STATUS_LABELS: Record<ValidationStatus, string> = {
  pending: "Pending",
  validating: "Validating",
  valid: "Valid",
  invalid: "Invalid",
  unsupported: "Unsupported",
  corrupted: "Corrupted",
  missing: "Missing",
};

export type AvailabilityStatus = "available" | "missing" | "moved" | "offline_volume" | "permission_denied";

export const AVAILABILITY_STATUS_LABELS: Record<AvailabilityStatus, string> = {
  available: "Available",
  missing: "Missing",
  moved: "Moved",
  offline_volume: "Offline volume",
  permission_denied: "Permission denied",
};

export type VideoPriority = "low" | "normal" | "high" | "urgent";

export const VIDEO_PRIORITIES: readonly VideoPriority[] = ["low", "normal", "high", "urgent"];

export const VIDEO_PRIORITY_LABELS: Record<VideoPriority, string> = {
  low: "Low",
  normal: "Normal",
  high: "High",
  urgent: "Urgent",
};

export type Orientation = "vertical" | "square" | "landscape";

export type VideoWarning = "missing_audio" | "low_resolution" | "zero_bitrate" | "extremely_short";

export const VIDEO_WARNING_LABELS: Record<VideoWarning, string> = {
  missing_audio: "No audio track",
  low_resolution: "Low resolution",
  zero_bitrate: "Zero bitrate",
  extremely_short: "Extremely short",
};

export interface Video {
  id: UUID;
  workspace_id: UUID;
  channel_id: UUID | null;
  source_id: UUID;
  original_filename: string;
  display_title: string;
  file_path: string;
  file_size_bytes: number;
  extension: string;
  duration_ms: number | null;
  width: number | null;
  height: number | null;
  fps: number | null;
  video_codec: string | null;
  audio_codec: string | null;
  bitrate: number | null;
  has_audio: boolean | null;
  content_hash: string | null;
  perceptual_hash: string | null;
  thumbnail_path: string | null;
  validation_status: ValidationStatus;
  availability_status: AvailabilityStatus;
  duplicate_of: UUID | null;
  priority: VideoPriority;
  notes: string | null;
  archived: boolean;
  created_at: ISODateTime;
  imported_at: ISODateTime;
  last_seen_at: ISODateTime;
  updated_at: ISODateTime;
}

export type MatchType = "exact" | "possible";

export interface DuplicateMatch {
  id: UUID;
  video_id: UUID;
  matched_video_id: UUID;
  similarity: number;
  match_type: MatchType;
  created_at: ISODateTime;
}

export interface VideoDetail extends Video {
  orientation: Orientation | null;
  aspect_ratio: string | null;
  warnings: VideoWarning[];
  possible_duplicates: DuplicateMatch[];
}

export type VideoSort =
  "newest_imported" | "oldest_imported" | "filename" | "duration" | "file_size" | "channel";

export interface ContentListRequest {
  search?: string;
  channel_id?: UUID;
  unassigned_only?: boolean;
  source_id?: UUID;
  validation_status?: ValidationStatus;
  availability_status?: AvailabilityStatus;
  orientation?: Orientation;
  priority?: VideoPriority;
  possible_duplicates_only?: boolean;
  include_archived?: boolean;
  sort?: VideoSort;
  page?: number;
  page_size?: number;
}

export interface VideoPage {
  items: Video[];
  total: number;
  page: number;
  page_size: number;
}

export interface VideoLibrarySummary {
  total: number;
  ready: number;
  processing: number;
  duplicates: number;
  invalid: number;
  missing: number;
  archived: number;
  unassigned: number;
}

/** Mirrors the tri-state `Option<Option<T>>` fields on the Rust side —
 * omit the key entirely to leave it untouched, send `null` to clear it,
 * or send a value to set it. */
export interface UpdateVideoInput {
  display_title?: string;
  channel_id?: UUID | null;
  priority?: VideoPriority;
  notes?: string | null;
}

export interface BulkUpdateInput {
  channel_id?: UUID | null;
  priority?: VideoPriority;
  archived?: boolean;
}

export type VideoSourceType = "cutpro_folder" | "manual_import" | "watch_folder";

export interface VideoSource {
  id: UUID;
  workspace_id: UUID;
  name: string;
  source_type: VideoSourceType;
  folder_path: string | null;
  channel_id: UUID | null;
  enabled: boolean;
  recursive: boolean;
  watch_enabled: boolean;
  created_at: ISODateTime;
  updated_at: ISODateTime;
  last_scan_at: ISODateTime | null;
  last_error: string | null;
}

export interface SourceSummary extends VideoSource {
  files_indexed: number;
}

export interface CreateSourceInput {
  name: string;
  source_type: Exclude<VideoSourceType, "manual_import">;
  folder_path: string;
  channel_id?: UUID;
  recursive: boolean;
  watch_enabled: boolean;
}

export interface UpdateSourceInput {
  name?: string;
  channel_id?: UUID | null;
  enabled?: boolean;
  recursive?: boolean;
  watch_enabled?: boolean;
}

export interface RejectedImport {
  path: string;
  reason: string;
}

export interface ImportSummary {
  imported: number;
  duplicates: number;
  moved: number;
  already_indexed: number;
  rejected: RejectedImport[];
}

export interface ReconcileSummary {
  scanned: number;
  imported: number;
  already_indexed: number;
  marked_missing: number;
  rejected: number;
}

export interface CacheInfo {
  database_size_bytes: number;
  thumbnail_cache_size_bytes: number;
  temp_cache_size_bytes: number;
}

export const SUPPORTED_VIDEO_EXTENSIONS = ["mp4", "mov", "mkv", "webm"] as const;
