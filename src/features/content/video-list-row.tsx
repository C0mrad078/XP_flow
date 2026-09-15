import { MoreHorizontal, Play } from "lucide-react";

import { Checkbox } from "@/components/ui/checkbox";
import {
  ContextMenu,
  ContextMenuContent,
  ContextMenuItem,
  ContextMenuTrigger,
} from "@/components/ui/context-menu";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { IconButton } from "@/components/ui/icon-button";
import { formatDurationMs, formatResolution } from "@/lib/formatting/media";
import { cn } from "@/lib/utilities/cn";
import { thumbnailUrl } from "@/lib/utilities/media-url";
import { useContentStore } from "@/stores/content-store";
import type { Video } from "@/types/media";

import { VideoStatusBadge } from "./media-status-badge";
import { useVideoActions } from "./use-video-actions";

const GRID_COLS = "grid-cols-[1.5rem_2.5rem_1fr_9rem_6rem_7rem_9rem_8rem_2rem]";

export function VideoListHeader() {
  return (
    <div className={cn("grid items-center gap-3 px-2 pb-1.5", GRID_COLS)}>
      <span />
      <span />
      <span className="text-caption">Title</span>
      <span className="text-caption">Channel</span>
      <span className="text-caption">Duration</span>
      <span className="text-caption">Resolution</span>
      <span className="text-caption">Source</span>
      <span className="text-caption">Status</span>
      <span />
    </div>
  );
}

export function VideoListRow({
  video,
  channelName,
  sourceName,
  selected,
  onToggleSelect,
}: {
  video: Video;
  channelName?: string;
  sourceName?: string;
  selected: boolean;
  onToggleSelect: (id: string) => void;
}) {
  const openDetail = useContentStore((state) => state.openDetail);
  const openQuickPreview = useContentStore((state) => state.openQuickPreview);
  const actions = useVideoActions(video);

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <div
          className={cn(
            "group grid items-center gap-3 rounded-md px-2 py-1.5 transition-colors hover:bg-surface-hover",
            GRID_COLS,
            selected && "bg-primary/5",
          )}
        >
          <Checkbox checked={selected} onCheckedChange={() => onToggleSelect(video.id)} />

          <button
            type="button"
            onClick={() => openDetail(video.id)}
            className="relative block h-11 w-8 overflow-hidden rounded bg-surface-elevated"
          >
            {video.thumbnail_path ? (
              <img src={thumbnailUrl(video.id)} alt="" className="size-full object-cover" draggable={false} />
            ) : (
              <div className="flex size-full items-center justify-center text-muted">
                <Play className="size-3" />
              </div>
            )}
          </button>

          <button
            type="button"
            onClick={() => openDetail(video.id)}
            className="truncate text-left text-body-small font-medium text-foreground"
          >
            {video.display_title}
          </button>

          <span className="truncate text-body-small text-muted-foreground">
            {channelName ?? "Unassigned"}
          </span>
          <span className="font-mono-data text-body-small text-muted-foreground">
            {formatDurationMs(video.duration_ms)}
          </span>
          <span className="font-mono-data text-body-small text-muted-foreground">
            {formatResolution(video.width, video.height)}
          </span>
          <span className="truncate text-body-small text-muted-foreground">{sourceName ?? "—"}</span>
          <VideoStatusBadge video={video} className="w-fit px-1.5 py-0 text-[0.625rem]" />

          <div className="flex justify-end opacity-0 group-hover:opacity-100">
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <IconButton label="More actions" size="sm">
                  <MoreHorizontal className="size-3.5" />
                </IconButton>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem onSelect={() => openQuickPreview(video.id)}>
                  <Play className="size-3.5" />
                  Preview
                </DropdownMenuItem>
                {actions.map((action) => (
                  <DropdownMenuItem
                    key={action.key}
                    onSelect={action.onSelect}
                    className={cn(action.destructive && "text-danger")}
                  >
                    <action.icon className="size-3.5" />
                    {action.label}
                  </DropdownMenuItem>
                ))}
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
      </ContextMenuTrigger>
      <ContextMenuContent>
        {actions.map((action) => (
          <ContextMenuItem
            key={action.key}
            onSelect={action.onSelect}
            className={cn(action.destructive && "text-danger")}
          >
            <action.icon className="size-3.5" />
            {action.label}
          </ContextMenuItem>
        ))}
      </ContextMenuContent>
    </ContextMenu>
  );
}
