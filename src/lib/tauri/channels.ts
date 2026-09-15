import { invoke } from "./client";
import type { UUID } from "@/types/domain";

/** Minimal real channel shape (mirrors `domain::channel::Channel`) — just
 * enough for the Phase 2 channel-assignment pickers. The full Channels
 * screen still renders from mock data (Phase 1 scope). */
export interface Channel {
  id: UUID;
  workspace_id: UUID;
  name: string;
  niche: string | null;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export const channelsApi = {
  list: (workspaceId: UUID) => invoke<Channel[]>("list_channels", { workspaceId }),
  create: (workspaceId: UUID, name: string) => invoke<Channel>("create_channel", { workspaceId, name }),
};
