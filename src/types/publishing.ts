import type { ISODateTime, Platform, UUID } from "./domain";

export type AttemptStatus = "pending" | "running" | "succeeded" | "failed" | "cancelled";
export type RateLimitOperation = "publish" | "status" | "auth" | "comments" | "analytics";
export type TemplateKind = "title" | "description";
export type ApprovalSource = "manual_schedule" | "add_to_queue" | "bulk_approval" | "publish_now";
export type ReadinessIssue =
  | "account_not_connected"
  | "account_reauth_required"
  | "publish_permission_missing"
  | "video_missing"
  | "video_invalid"
  | "metadata_invalid"
  | "consent_required"
  | "platform_not_approved"
  | "rate_limited"
  | "publication_locked"
  | "already_published";

export interface PublicationAttempt {
  id: UUID;
  publication_id: UUID;
  attempt_number: number;
  provider: Platform;
  status: AttemptStatus;
  started_at: ISODateTime | null;
  completed_at: ISODateTime | null;
  bytes_total: number | null;
  bytes_uploaded: number | null;
  error_code: string | null;
  error_message: string | null;
  retryable: boolean | null;
  remote_operation_id: string | null;
  created_at: ISODateTime;
}
export interface RenderedMetadata {
  title: string;
  description: string;
  hashtags: string[];
  provider_options: Record<string, unknown>;
}
export interface MetadataValidationIssue {
  code: string;
  message: string;
}
export interface MetadataTemplate {
  id: UUID;
  workspace_id: UUID;
  channel_id: UUID | null;
  platform: Platform | null;
  kind: TemplateKind;
  template_text: string;
  created_at: ISODateTime;
  updated_at: ISODateTime;
}
export interface HashtagSet {
  id: UUID;
  workspace_id: UUID;
  channel_id: UUID | null;
  platform: Platform | null;
  name: string;
  hashtags: string[];
  created_at: ISODateTime;
  updated_at: ISODateTime;
}
export interface ProviderRateState {
  operation: RateLimitOperation;
  limited_until: ISODateTime | null;
}
export interface PublishingSettings {
  enabled: boolean;
  paused: boolean;
  max_concurrent_uploads: number;
  missed_schedule_policy: "publish_within_grace" | "needs_review" | "skip";
  missed_schedule_grace_period_minutes: number;
}
export interface MetadataFieldUpdate<T> {
  set: boolean;
  value?: T | null;
}
export interface UpdatePublicationMetadataInput {
  title?: string;
  description?: string;
  hashtags?: string[];
  title_override?: MetadataFieldUpdate<string>;
  description_override?: MetadataFieldUpdate<string>;
  hashtags_override?: MetadataFieldUpdate<string[]>;
  title_template_id?: MetadataFieldUpdate<UUID>;
  description_template_id?: MetadataFieldUpdate<UUID>;
  hashtag_set_id?: MetadataFieldUpdate<UUID>;
  provider_options_override?: MetadataFieldUpdate<Record<string, unknown>>;
}
export interface UpdatePublishingSettingsInput {
  enabled?: boolean;
  paused?: boolean;
  max_concurrent_uploads?: number;
  missed_schedule_policy?: PublishingSettings["missed_schedule_policy"];
  missed_schedule_grace_period_minutes?: number;
}
