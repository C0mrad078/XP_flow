import { create } from "zustand";

import type { UUID } from "@/types/domain";
import type { PublishProgressEvent } from "@/types/publishing";

interface PublishProgressState {
  /** Keyed by publication_id — the latest event seen for each one.
   * Deliberately not TanStack Query state (section 29): this is
   * transient, per-viewer presentation only, never treated as the
   * authoritative result. */
  progress: Record<UUID, PublishProgressEvent>;
  record: (event: PublishProgressEvent) => void;
  /** Clears a publication's progress once it leaves an active state, so
   * a stale bar never lingers after a publication finishes. */
  clear: (publicationId: UUID) => void;
}

export const usePublishProgressStore = create<PublishProgressState>((set) => ({
  progress: {},
  record: (event) => set((state) => ({ progress: { ...state.progress, [event.publication_id]: event } })),
  clear: (publicationId) =>
    set((state) => {
      if (!(publicationId in state.progress)) return state;
      const next = { ...state.progress };
      delete next[publicationId];
      return { progress: next };
    }),
}));
