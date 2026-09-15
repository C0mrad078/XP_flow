import { invoke } from "./client";
import type { Platform, UUID } from "@/types/domain";
import type { PlatformAccount } from "@/types/scheduling";

export const platformAccountsApi = {
  listForChannel: (channelId: UUID) => invoke<PlatformAccount[]>("list_platform_accounts", { channelId }),
  create: (channelId: UUID, platform: Platform) =>
    invoke<PlatformAccount>("create_platform_account", { channelId, platform }),
  setDefault: (id: UUID) => invoke<PlatformAccount>("set_default_platform_account", { id }),
};
