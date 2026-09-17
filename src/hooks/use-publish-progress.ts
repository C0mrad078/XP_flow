import { useEffect } from "react";

import { subscribeToPublishProgress } from "@/lib/tauri/publish-progress";
import { usePublishProgressStore } from "@/stores/publish-progress-store";
import type { UUID } from "@/types/domain";

/**
 * Mounts the single, app-wide subscription to the backend's live upload
 * progress event (section 28/73). Mounted once at the app root
 * (`App.tsx`) — every screen reads from the shared store instead of
 * opening its own listener, so there is never more than one subscription
 * regardless of how many components care about progress. Cleans up on
 * unmount, which in practice only happens on app teardown.
 */
export function usePublishProgressListener(): void {
  const record = usePublishProgressStore((state) => state.record);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    subscribeToPublishProgress((event) => record(event))
      .then((fn) => {
        if (cancelled) {
          fn();
        } else {
          unlisten = fn;
        }
      })
      .catch(() => {
        // No Tauri runtime available (e.g. a plain browser preview) —
        // progress simply never arrives; nothing else depends on it.
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [record]);
}

/** The latest live progress for one publication, or `undefined` if none
 * has arrived yet (e.g. before the first chunk, or if it never uploads
 * again after this session started). Never authoritative on its own —
 * callers still derive final status from the persisted `Publication`. */
export function usePublicationProgress(publicationId: UUID | null) {
  return usePublishProgressStore((state) => (publicationId ? state.progress[publicationId] : undefined));
}
