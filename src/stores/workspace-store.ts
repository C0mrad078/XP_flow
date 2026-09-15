import { create } from "zustand";

import { workspaceApi } from "@/lib/tauri";
import type { Workspace } from "@/types/domain";

interface WorkspaceState {
  workspace: Workspace | null;
  isLoading: boolean;
  isLoaded: boolean;
  load: () => Promise<void>;
  createWorkspace: (name: string) => Promise<Workspace>;
}

export const useWorkspaceStore = create<WorkspaceState>((set, get) => ({
  workspace: null,
  isLoading: false,
  isLoaded: false,

  load: async () => {
    if (get().isLoaded || get().isLoading) return;
    set({ isLoading: true });
    try {
      const workspace = await workspaceApi.getCurrent();
      set({ workspace, isLoading: false, isLoaded: true });
    } catch {
      // No backend / no workspace yet — fall through to onboarding rather
      // than leaving the app stuck on the startup loading screen forever.
      set({ workspace: null, isLoading: false, isLoaded: true });
    }
  },

  createWorkspace: async (name: string) => {
    const workspace = await workspaceApi.create(name);
    set({ workspace, isLoaded: true });
    return workspace;
  },
}));
