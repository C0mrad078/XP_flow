import { useEffect } from "react";

import { useContentStore } from "@/stores/content-store";

function isTextInputTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || target.isContentEditable;
}

/** Section 43: Space opens Quick Preview for the current selection (a
 * single checked video, or whichever video's Details panel is open);
 * Escape closes it. Skipped entirely while focus is in a text field so it
 * never fights typing a search query or a note. */
export function useQuickPreviewKeys() {
  const quickPreviewVideoId = useContentStore((state) => state.quickPreviewVideoId);
  const openQuickPreview = useContentStore((state) => state.openQuickPreview);
  const closeQuickPreview = useContentStore((state) => state.closeQuickPreview);
  const detailVideoId = useContentStore((state) => state.detailVideoId);
  const selectedIds = useContentStore((state) => state.selectedIds);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (isTextInputTarget(event.target)) return;

      if (event.key === "Escape" && quickPreviewVideoId) {
        event.preventDefault();
        closeQuickPreview();
        return;
      }

      if (event.code === "Space" && !quickPreviewVideoId) {
        const target = detailVideoId ?? (selectedIds.length === 1 ? selectedIds[0] : undefined);
        if (target) {
          event.preventDefault();
          openQuickPreview(target);
        }
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [quickPreviewVideoId, detailVideoId, selectedIds, openQuickPreview, closeQuickPreview]);
}
