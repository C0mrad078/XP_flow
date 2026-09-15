import { invoke } from "./client";
import type { Platform, UUID } from "@/types/domain";
import type { AuthFlowState, PlatformAccount } from "@/types/platform-auth";

export const platformAuthApi = {
  beginConnect: (workspaceId: UUID, channelId: UUID, platform: Platform) =>
    invoke<UUID>("begin_platform_connect", { workspaceId, channelId, platform }),
  beginReconnect: (accountId: UUID, allowIdentityChange: boolean) =>
    invoke<UUID>("begin_platform_reconnect", { accountId, allowIdentityChange }),
  pollStatus: (sessionId: UUID) =>
    invoke<AuthFlowState | null>("poll_platform_connect_status", { sessionId }),
  cancel: (sessionId: UUID) => invoke<void>("cancel_platform_connect", { sessionId }),
  validate: (accountId: UUID) => invoke<PlatformAccount>("validate_platform_account", { accountId }),
  refresh: (accountId: UUID) => invoke<PlatformAccount>("refresh_platform_account", { accountId }),
  disconnect: (accountId: UUID) => invoke<PlatformAccount>("disconnect_platform_account", { accountId }),
};
