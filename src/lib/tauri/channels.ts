import { invoke } from "./client";
import type { UUID } from "@/types/domain";
import type { PlatformAccount } from "@/types/platform-auth";

export type ChannelStatus = "active" | "paused";

/** Mirrors `domain::channel::Channel`. */
export interface Channel {
  id: UUID;
  workspace_id: UUID;
  name: string;
  niche: string | null;
  description: string | null;
  status: ChannelStatus;
  created_at: string;
  updated_at: string;
}

/** Mirrors `application::channel_service::ChannelOverview` (section 96) —
 * one aggregate query set backs every field here, regardless of how many
 * channels the workspace has. */
export interface ChannelOverview {
  channel: Channel;
  queued_count: number;
  active_slot_count: number;
  platform_accounts: PlatformAccount[];
}

export const channelsApi = {
  list: (workspaceId: UUID) => invoke<Channel[]>("list_channels", { workspaceId }),
  create: (workspaceId: UUID, name: string) => invoke<Channel>("create_channel", { workspaceId, name }),
  get: (id: UUID) => invoke<Channel | null>("get_channel", { id }),
  setStatus: (id: UUID, status: ChannelStatus) => invoke<Channel>("set_channel_status", { id, status }),
  operationalOverview: (workspaceId: UUID) =>
    invoke<ChannelOverview[]>("get_channel_operational_overview", { workspaceId }),
};
