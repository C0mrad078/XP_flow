/**
 * Frontend mirror of the Rust Phase 3 queue/scheduler types serialized
 * across IPC (src-tauri/src/domain/{schedule_slot,schedule_exception,
 * publication_query,platform_account}.rs). Field names and casing must
 * match serde's output exactly.
 */
import type { ISODateTime, Platform, Publication, PublicationStatus, UUID } from "./domain";
import type { VideoPriority } from "./media";

export interface ScheduleSlot {
  id: UUID;
  channel_id: UUID;
  /** null = channel-default slot (applies to any platform without its own
   * override); a specific platform overrides the default for that weekday. */
  platform: Platform | null;
  /** 0 = Monday .. 6 = Sunday. */
  day_of_week: number;
  /** "HH:MM" 24h, in the workspace's timezone. */
  time_of_day: string;
  is_active: boolean;
  created_at: ISODateTime;
  updated_at: ISODateTime;
}

export type ScheduleExceptionKind = "skip";

export interface ScheduleException {
  id: UUID;
  channel_id: UUID;
  /** "YYYY-MM-DD". */
  date: string;
  kind: ScheduleExceptionKind;
  reason: string | null;
  created_at: ISODateTime;
}

/** A publication placed on the calendar, with its UTC `scheduled_at`
 * already resolved into the workspace's local date/time. */
export interface CalendarPublication {
  publication: Publication;
  /** "YYYY-MM-DD" in the workspace's local timezone. */
  local_date: string;
  /** "HH:MM" in the workspace's local timezone. */
  local_time: string;
}

export type { Capability, ConnectionHealth, PlatformAccount, PlatformAccountStatus } from "./platform-auth";

export type QueueSort = "queue_order" | "priority_desc" | "newest_first" | "oldest_first";

export interface PublicationListRequest {
  search?: string;
  channel_id?: UUID;
  platform_account_id?: UUID;
  platform?: Platform;
  priority?: VideoPriority;
  statuses?: PublicationStatus[];
  requires_attention?: boolean;
  sort?: QueueSort;
  page?: number;
  page_size?: number;
}

export interface PublicationPage {
  items: Publication[];
  total: number;
  page: number;
  page_size: number;
}

export interface AddToQueueOutcome {
  video_id: UUID;
  publication: Publication | null;
  error: { code: string; user_message: string; developer_message: string } | null;
}

export interface BulkScheduleResult {
  scheduled: Publication[];
  skipped_count: number;
}
