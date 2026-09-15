import { useState } from "react";
import { Archive, ArchiveRestore, ListPlus, RefreshCw, X } from "lucide-react";

import { ChannelPicker } from "@/components/common/channel-picker";
import { Button } from "@/components/ui/button";
import { IconButton } from "@/components/ui/icon-button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { AddToQueueDialog } from "@/features/queue/add-to-queue-dialog";
import { useBulkUpdateVideos } from "@/hooks/use-content";
import { toast } from "@/stores/toast-store";
import { useContentStore } from "@/stores/content-store";
import { VIDEO_PRIORITIES, VIDEO_PRIORITY_LABELS } from "@/types/media";
import { isAppError } from "@/types/domain";

/** Section 47 — appears once at least one video is selected. */
export function BulkActionBar() {
  const selectedIds = useContentStore((state) => state.selectedIds);
  const clearSelection = useContentStore((state) => state.clearSelection);
  const bulkUpdate = useBulkUpdateVideos();
  const [addToQueueOpen, setAddToQueueOpen] = useState(false);

  if (selectedIds.length === 0) return null;

  function run(label: string, payload: Parameters<typeof bulkUpdate.mutate>[0]["input"]) {
    bulkUpdate.mutate(
      { ids: selectedIds, input: payload },
      {
        onSuccess: (count) => {
          toast({
            variant: "success",
            title: label,
            description: `${count} video${count === 1 ? "" : "s"} updated`,
          });
          clearSelection();
        },
        onError: (error) =>
          toast({
            variant: "error",
            title: `${label} failed`,
            description: isAppError(error) ? error.user_message : undefined,
          }),
      },
    );
  }

  return (
    <div className="sticky bottom-4 z-(--z-sticky) mx-auto flex w-fit items-center gap-3 rounded-xl border border-border bg-surface-elevated px-4 py-2.5 shadow-xl">
      <span className="text-body-small font-medium text-foreground">{selectedIds.length} selected</span>

      <div className="h-5 w-px bg-border" />

      <Button variant="secondary" size="sm" onClick={() => setAddToQueueOpen(true)}>
        <ListPlus />
        Add to queue
      </Button>
      <AddToQueueDialog
        videoIds={selectedIds}
        open={addToQueueOpen}
        onOpenChange={setAddToQueueOpen}
        onDone={clearSelection}
      />

      <div className="h-5 w-px bg-border" />

      <div className="flex items-center gap-1.5 text-body-small text-muted-foreground">
        Channel
        <ChannelPicker
          value={undefined}
          onChange={(channelId) => run("Channel assigned", { channel_id: channelId })}
          className="w-36"
        />
      </div>

      <div className="flex items-center gap-1.5 text-body-small text-muted-foreground">
        Priority
        <Select
          onValueChange={(value) =>
            run("Priority updated", { priority: value as (typeof VIDEO_PRIORITIES)[number] })
          }
        >
          <SelectTrigger className="h-8 w-28">
            <SelectValue placeholder="Set…" />
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

      <Button variant="ghost" size="sm" onClick={() => run("Archived", { archived: true })}>
        <Archive />
        Archive
      </Button>
      <Button variant="ghost" size="sm" onClick={() => run("Unarchived", { archived: false })}>
        <ArchiveRestore />
        Unarchive
      </Button>

      {bulkUpdate.isPending && <RefreshCw className="size-3.5 animate-spin text-muted-foreground" />}

      <IconButton label="Clear selection" size="sm" onClick={clearSelection}>
        <X className="size-4" />
      </IconButton>
    </div>
  );
}
