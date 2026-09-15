import { Dialog, DialogContent } from "@/components/ui/dialog";
import { useContentStore } from "@/stores/content-store";

import { VideoPlayer } from "./video-player";

/** Section 43 — Space opens a quick preview of the selected video, Escape
 * closes it. The actual keydown listener lives in `use-quick-preview-keys`
 * so it can be scoped to the Content page and skip text-input contexts. */
export function QuickPreviewOverlay() {
  const videoId = useContentStore((state) => state.quickPreviewVideoId);
  const close = useContentStore((state) => state.closeQuickPreview);

  return (
    <Dialog open={Boolean(videoId)} onOpenChange={(open) => !open && close()}>
      <DialogContent className="max-w-3xl border-none bg-transparent p-0 shadow-none" hideClose>
        {videoId && (
          <div className="overflow-hidden rounded-lg bg-black">
            <VideoPlayer videoId={videoId} autoPlay className="max-h-[80vh]" />
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
