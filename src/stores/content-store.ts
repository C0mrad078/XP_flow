import { create } from "zustand";
import { persist } from "zustand/middleware";

import type { AvailabilityStatus, Orientation, ValidationStatus, VideoPriority, VideoSort } from "@/types/media";

export type ContentViewMode = "grid" | "list";

interface ContentFilters {
  search: string;
  channelId?: string;
  unassignedOnly: boolean;
  sourceId?: string;
  validationStatus?: ValidationStatus;
  availabilityStatus?: AvailabilityStatus;
  orientation?: Orientation;
  priority?: VideoPriority;
  possibleDuplicatesOnly: boolean;
  includeArchived: boolean;
  sort: VideoSort;
  page: number;
}

const DEFAULT_FILTERS: ContentFilters = {
  search: "",
  unassignedOnly: false,
  possibleDuplicatesOnly: false,
  includeArchived: false,
  sort: "newest_imported",
  page: 0,
};

interface ContentState {
  viewMode: ContentViewMode;
  setViewMode: (mode: ContentViewMode) => void;

  filters: ContentFilters;
  setFilters: (patch: Partial<ContentFilters>) => void;
  resetFilters: () => void;

  selectedIds: string[];
  toggleSelected: (id: string) => void;
  selectMany: (ids: string[]) => void;
  clearSelection: () => void;

  detailVideoId: string | null;
  openDetail: (id: string) => void;
  closeDetail: () => void;

  quickPreviewVideoId: string | null;
  openQuickPreview: (id: string) => void;
  closeQuickPreview: () => void;
}

export const useContentStore = create<ContentState>()(
  persist(
    (set, get) => ({
      viewMode: "grid",
      setViewMode: (mode) => set({ viewMode: mode }),

      filters: DEFAULT_FILTERS,
      setFilters: (patch) =>
        set((state) => ({
          // Any filter change other than an explicit page bump resets to page 0.
          filters: { ...state.filters, ...patch, page: patch.page ?? 0 },
        })),
      resetFilters: () => set({ filters: DEFAULT_FILTERS }),

      selectedIds: [],
      toggleSelected: (id) =>
        set((state) => ({
          selectedIds: state.selectedIds.includes(id) ? state.selectedIds.filter((x) => x !== id) : [...state.selectedIds, id],
        })),
      selectMany: (ids) => set({ selectedIds: ids }),
      clearSelection: () => set({ selectedIds: [] }),

      detailVideoId: null,
      openDetail: (id) => set({ detailVideoId: id }),
      closeDetail: () => set({ detailVideoId: null }),

      quickPreviewVideoId: null,
      openQuickPreview: (id) => {
        if (get().detailVideoId) return; // don't fight the details panel's own preview
        set({ quickPreviewVideoId: id });
      },
      closeQuickPreview: () => set({ quickPreviewVideoId: null }),
    }),
    {
      name: "xpflow-content-ui",
      partialize: (state) => ({ viewMode: state.viewMode }),
    },
  ),
);
