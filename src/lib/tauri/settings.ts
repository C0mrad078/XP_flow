import { invoke } from "./client";
import type { AppSettings, ThemePreference } from "@/types/domain";

export interface UpdateSettingsInput {
  theme?: ThemePreference;
  launch_on_startup?: boolean;
}

export const settingsApi = {
  get: () => invoke<AppSettings>("get_settings"),
  update: (input: UpdateSettingsInput) => invoke<AppSettings>("update_settings", { input }),
};
