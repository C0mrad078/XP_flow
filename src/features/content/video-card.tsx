import { MoreHorizontal, Play, Plus } from "lucide-react";

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
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { formatDate } from "@/lib/formatting/date";
import { formatDurationMs, formatResolution } from "@/lib/formatting/media";
import { cn } from "@/lib/utilities/cn";
import { thumbnailUrl } from "@/lib/utilities/media-url";
import { useContentStore } from "@/stores/content-store";
import type { Video } from "@/types/media";

import { VideoStatusBadge } from "./media-status-badge";
import { useVideoActions } from "./use-video-actions";

export interface VideoCardProps {
  video: Video;
  channelName?: string;
  selected: boolean;
  onToggleSelect: (id: string) => void;
}

export function VideoCard({ video, channelName, selected, onToggleSelect }: VideoCardProps) {
  const openDetail = useContentStore((state) => state.openDetail);
  const openQuickPreview = useContentStore((state) => state.openQuickPreview);
  const actions = useVideoActions(video);

  return (
    <ContextMenu>
      <ContextMenuTrigger asChild>
        <div
          className={cn(
            "group relative flex flex-col gap-2 rounded-lg border border-border bg-surface p-2 transition-colors",
            "hover:border-border-strong",
            selected && "border-primary ring-1 ring-primary",
          )}
        >
          <div className="relative">
            <button
              type="button"
              onClick={() => openDetail(video.id)}
              className="block aspect-9/16 w-full overflow-hidden rounded-md bg-surface-elevated"
            >
              {video.thumbnail_path ? (
                <img
                  src={thumbnailUrl(video.id)}
                  alt=""
                  className="size-full object-cover"
                  draggable={false}
                />
              ) : (
                <div className="flex size-full items-center justify-center text-muted">
                  <Play className="size-5" />
                </div>
              )}
            </button>

            <span className="absolute bottom-1 right-1 rounded bg-black/60 px-1 py-0.5 font-mono-data text-[0.625rem] text-white">
              {formatDurationMs(video.duration_ms)}
            </span>

            <div className="absolute left-1.5 top-1.5">
              <Checkbox
                checked={selected}
                onCheckedChange={() => onToggleSelect(video.id)}
                className={cn(
                  "bg-black/40 border-white/40",
                  "data-[state=unchecked]:opacity-0 group-hover:opacity-100",
                  selected && "opacity-100",
                )}
                onClick={(e) => e.stopPropagation()}
              />
            </div>

            <div className="absolute inset-x-1.5 top-1.5 flex justify-end opacity-0 transition-opacity group-hover:opacity-100">
              <div className="flex items-center gap-1">
                <Tooltip>
                  <TooltipTrigger asChild>
                    <IconButton
                      label="Preview"
                      size="sm"
                      className="bg-black/40 text-white hover:bg-black/60"
                      onClick={() => openQuickPreview(video.id)}
                    >
                      <Play className="size-3.5" />
                    </IconButton>
                  </TooltipTrigger>
                  <TooltipContent>Preview (Space)</TooltipContent>
                </Tooltip>
                <Tooltip>
                  <TooltipTrigger asChild>
                    <IconButton
                      label="Add to queue"
                      size="sm"
                      className="bg-black/40 text-white hover:bg-black/60"
                      disabled
                    >
                      <Plus className="size-3.5" />
                    </IconButton>
                  </TooltipTrigger>
                  <TooltipContent>Add to queue — coming in Phase 3</TooltipContent>
                </Tooltip>
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <IconButton
                      label="More actions"
                      size="sm"
                      className="bg-black/40 text-white hover:bg-black/60"
                    >
                      <MoreHorizontal className="size-3.5" />
                    </IconButton>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end">
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
          </div>

          <div className="flex flex-col gap-1 px-0.5">
            <p className="truncate text-body-small font-medium text-foreground" title={video.display_title}>
              {video.display_title}
            </p>
            <p className="text-caption normal-case tracking-normal">{channelName ?? "Unassigned"}</p>
            <div className="flex items-center justify-between pt-0.5">
              <VideoStatusBadge video={video} className="px-1.5 py-0 text-[0.625rem]" />
              <span className="text-caption normal-case tracking-normal">
                {formatResolution(video.width, video.height)}
              </span>
            </div>
            <p className="text-caption normal-case tracking-normal">
              Imported {formatDate(video.imported_at)}
            </p>
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
