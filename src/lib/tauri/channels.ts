import { invoke } from "./client";
import type { UUID } from "@/types/domain";

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

export const channelsApi = {
  list: (workspaceId: UUID) => invoke<Channel[]>("list_channels", { workspaceId }),
  create: (workspaceId: UUID, name: string) => invoke<Channel>("create_channel", { workspaceId, name }),
  get: (id: UUID) => invoke<Channel | null>("get_channel", { id }),
  setStatus: (id: UUID, status: ChannelStatus) => invoke<Channel>("set_channel_status", { id, status }),
};
