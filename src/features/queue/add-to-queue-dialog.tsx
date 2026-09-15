import { useState } from "react";

import { ChannelPicker } from "@/components/common/channel-picker";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { useAddToQueueBulk } from "@/hooks/use-queue";
import { PLATFORMS, PLATFORM_LABELS, type Platform, type UUID } from "@/types/domain";
import { VIDEO_PRIORITIES, VIDEO_PRIORITY_LABELS, type VideoPriority } from "@/types/media";
import { toast } from "@/stores/toast-store";

export interface AddToQueueDialogProps {
  videoIds: UUID[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onDone?: () => void;
}

/** Section 36's "Add to Queue" flow: pick a target channel, platform and
 * (optional) priority override, then create one `Publication` per video.
 * Works uniformly for a single video or a bulk selection — the backend's
 * `add_to_queue_bulk` reports a per-item outcome rather than failing the
 * whole batch on the first duplicate. */
export function AddToQueueDialog({ videoIds, open, onOpenChange, onDone }: AddToQueueDialogProps) {
  const [channelId, setChannelId] = useState<UUID | null>(null);
  const [platform, setPlatform] = useState<Platform>("youtube");
  const [priority, setPriority] = useState<VideoPriority | "">("");
  const addToQueue = useAddToQueueBulk();

  function handleSubmit() {
    if (!channelId) return;
    addToQueue.mutate(
      { videoIds, channelId, platform, priority: priority || null },
      {
        onSuccess: (outcomes) => {
          const succeeded = outcomes.filter((o) => o.publication !== null).length;
          const failed = outcomes.length - succeeded;
          toast({
            variant: failed === 0 ? "success" : "warning",
            title: `Added ${succeeded} video${succeeded === 1 ? "" : "s"} to the ${PLATFORM_LABELS[platform]} queue`,
            description:
              failed > 0 ? `${failed} skipped (already queued for this channel/platform)` : undefined,
          });
          onOpenChange(false);
          onDone?.();
        },
        onError: () => toast({ variant: "error", title: "Couldn't add to queue" }),
      },
    );
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-sm">
        <DialogHeader>
          <DialogTitle>Add to Queue</DialogTitle>
          <DialogDescription>
            {videoIds.length} video{videoIds.length === 1 ? "" : "s"} will be queued for a channel and
            platform.
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-3">
          <div className="flex flex-col gap-1.5">
            <span className="text-body-small font-medium text-foreground">Channel</span>
            <ChannelPicker value={channelId} onChange={setChannelId} allowUnassigned={false} />
          </div>

          <div className="flex flex-col gap-1.5">
            <span className="text-body-small font-medium text-foreground">Platform</span>
            <Select value={platform} onValueChange={(value) => setPlatform(value as Platform)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {PLATFORMS.map((p) => (
                  <SelectItem key={p} value={p}>
                    {PLATFORM_LABELS[p]}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="flex flex-col gap-1.5">
            <span className="text-body-small font-medium text-foreground">Priority (optional)</span>
            <Select value={priority} onValueChange={(value) => setPriority(value as VideoPriority)}>
              <SelectTrigger>
                <SelectValue placeholder="Use each video's own priority" />
              </SelectTrigger>
              <SelectContent>
                {VIDEO_PRIORITIES.map((p) => (
                  <SelectItem key={p} value={p}>
                    {VIDEO_PRIORITY_LABELS[p]}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={!channelId || addToQueue.isPending}>
            Add to Queue
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
