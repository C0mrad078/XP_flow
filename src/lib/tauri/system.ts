import { invoke } from "./client";
import type { AppInfo, MediaToolchainStatus } from "@/types/domain";
import type { CacheInfo } from "@/types/media";

export const systemApi = {
  getAppInfo: () => invoke<AppInfo>("get_app_info"),
  getMediaStatus: () => invoke<MediaToolchainStatus>("get_media_status"),
  getCacheInfo: () => invoke<CacheInfo>("get_cache_info"),
  clearTempCache: () => invoke<number>("clear_temp_cache"),
};
