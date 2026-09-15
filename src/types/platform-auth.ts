/**
 * Frontend mirror of the Rust Phase 4 platform-authentication types
 * serialized across IPC (src-tauri/src/domain/{platform_account,
 * capability,connection_health,oauth/flow_state}.rs). Field names and
 * casing must match serde's output exactly.
 */
import type { ISODateTime, Platform, UUID } from "./domain";

export type PlatformAccountStatus =
  | "not_configured"
  | "connecting"
  | "connected"
  | "refreshing"
  | "permission_missing"
  | "reauth_required"
  | "revoked"
  | "error";

export type Capability =
  "read_profile" | "upload_video" | "read_video_status" | "read_metrics" | "read_comments" | "write_comments";

export const CAPABILITY_LABELS: Record<Capability, string> = {
  read_profile: "Profile",
  upload_video: "Publishing",
  read_video_status: "Video status",
  read_metrics: "Metrics",
  read_comments: "Read comments",
  write_comments: "Reply to comments",
};

/** Mirrors `domain::platform_account::PlatformAccount`. No OAuth secret
 * ever appears in this shape — those stay behind the OS keychain
 * (YouTube) or the Auth Broker (TikTok/Kwai), referenced only by an
 * opaque id. */
export interface PlatformAccount {
  id: UUID;
  workspace_id: UUID;
  channel_id: UUID;
  platform: Platform;
  provider_account_id: string | null;
  provider_connection_id: string | null;
  display_name: string | null;
  username_or_handle: string | null;
  avatar_url: string | null;
  status: PlatformAccountStatus;
  granted_scopes: string[];
  capabilities: Capability[];
  default_target: boolean;
  access_expires_at: ISODateTime | null;
  refresh_expires_at: ISODateTime | null;
  connected_at: ISODateTime | null;
  last_validated_at: ISODateTime | null;
  last_refreshed_at: ISODateTime | null;
  last_error_code: string | null;
  last_error_message: string | null;
  created_at: ISODateTime;
  updated_at: ISODateTime;
}

/** Derived, display-only — mirrors `domain::connection_health`. Never
 * persisted; recomputed from a `PlatformAccount` every time it's needed,
 * same "derive, don't store" discipline as Phase 3's overdue detection. */
export type ConnectionHealth =
  | "healthy"
  | "token_expiring"
  | "permission_missing"
  | "refresh_required"
  | "disconnected"
  | "provider_error";

export const CONNECTION_HEALTH_LABELS: Record<ConnectionHealth, string> = {
  healthy: "Healthy",
  token_expiring: "Token expiring",
  permission_missing: "Permission missing",
  refresh_required: "Needs reconnection",
  disconnected: "Not connected",
  provider_error: "Provider error",
};

/** Section 38's buffer, mirrored from `connection_health::TOKEN_EXPIRING_BUFFER`
 * (24h) — kept in sync manually since this is the one small piece of derived
 * logic duplicated client-side (see `deriveConnectionHealth`'s doc comment). */
const TOKEN_EXPIRING_BUFFER_MS = 24 * 60 * 60 * 1000;

/** Mirrors `domain::connection_health::derive_connection_health` exactly —
 * same rationale as Phase 3's `isPublicationOverdue`: a pure, cheap
 * computation the UI needs constantly is worth duplicating client-side
 * rather than adding a round trip for every row rendered. */
export function deriveConnectionHealth(account: PlatformAccount, now: Date = new Date()): ConnectionHealth {
  switch (account.status) {
    case "not_configured":
    case "revoked":
      return "disconnected";
    case "reauth_required":
      return "refresh_required";
    case "permission_missing":
      return "permission_missing";
    case "error":
      return "provider_error";
    case "connecting":
    case "refreshing":
    case "connected": {
      if (account.access_expires_at) {
        const expiresAt = new Date(account.access_expires_at).getTime();
        if (expiresAt <= now.getTime() + TOKEN_EXPIRING_BUFFER_MS) return "token_expiring";
      }
      return "healthy";
    }
  }
}

/** Mirrors `domain::oauth::flow_state::AuthFlowState` — the live,
 * in-progress state of a connect/reconnect attempt (distinct from
 * `PlatformAccountStatus`, which describes the persisted account). */
export type AuthFlowState =
  | { state: "opening_browser" }
  | { state: "waiting_for_authorization" }
  | { state: "verifying_account" }
  | { state: "saving_connection" }
  | { state: "connected" }
  | { state: "failed"; code: string; message: string };
