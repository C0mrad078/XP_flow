import { invoke } from "./client";
import type { AppInfo, MediaToolchainStatus } from "@/types/domain";

export const systemApi = {
  getAppInfo: () => invoke<AppInfo>("get_app_info"),
  getMediaStatus: () => invoke<MediaToolchainStatus>("get_media_status"),
};
