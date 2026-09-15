import { create } from "zustand";

import { settingsApi } from "@/lib/tauri";
import { applyTheme } from "@/lib/utilities/theme";
import type { AppSettings, ThemePreference } from "@/types/domain";

interface SettingsState {
  settings: AppSettings;
  isLoading: boolean;
  isLoaded: boolean;
  load: () => Promise<void>;
  setTheme: (theme: ThemePreference) => Promise<void>;
  setLaunchOnStartup: (launchOnStartup: boolean) => Promise<void>;
}

const defaultSettings: AppSettings = { theme: "dark", launch_on_startup: false };

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: defaultSettings,
  isLoading: false,
  isLoaded: false,

  load: async () => {
    if (get().isLoaded || get().isLoading) return;
    set({ isLoading: true });
    try {
      const settings = await settingsApi.get();
      applyTheme(settings.theme);
      set({ settings, isLoading: false, isLoaded: true });
    } catch {
      applyTheme(defaultSettings.theme);
      set({ isLoading: false, isLoaded: true });
    }
  },

  setTheme: async (theme) => {
    applyTheme(theme);
    set((state) => ({ settings: { ...state.settings, theme } }));
    const settings = await settingsApi.update({ theme });
    set({ settings });
  },

  setLaunchOnStartup: async (launchOnStartup) => {
    set((state) => ({ settings: { ...state.settings, launch_on_startup: launchOnStartup } }));
    const settings = await settingsApi.update({ launch_on_startup: launchOnStartup });
    set({ settings });
  },
}));
