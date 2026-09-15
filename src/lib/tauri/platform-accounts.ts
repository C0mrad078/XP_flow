import { invoke } from "./client";
import type { UUID } from "@/types/domain";
import type { PlatformAccount } from "@/types/platform-auth";

export const platformAccountsApi = {
  listForChannel: (channelId: UUID) => invoke<PlatformAccount[]>("list_platform_accounts", { channelId }),
  listForWorkspace: (workspaceId: UUID) =>
    invoke<PlatformAccount[]>("list_platform_accounts_for_workspace", { workspaceId }),
  setDefault: (id: UUID) => invoke<PlatformAccount>("set_default_platform_account", { id }),
  reassignChannel: (id: UUID, newChannelId: UUID) =>
    invoke<PlatformAccount>("reassign_platform_account_channel", { id, newChannelId }),
};
