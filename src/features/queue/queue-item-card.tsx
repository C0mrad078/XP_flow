import { Lock, TriangleAlert } from "lucide-react";

import { MediaThumbnail } from "@/components/common/media-thumbnail";
import { Badge } from "@/components/ui/badge";
import { PlatformBadge } from "@/components/ui/platform-badge";
import { StatusBadge } from "@/components/ui/status-badge";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { usePublicationProgress } from "@/hooks/use-publish-progress";
import { formatTimeInZone } from "@/lib/formatting/date";
import { cn } from "@/lib/utilities/cn";
import { VIDEO_PRIORITY_LABELS, type VideoPriority } from "@/types/media";
import type { Publication } from "@/types/domain";

const PRIORITY_VARIANT: Record<VideoPriority, "outline" | "default" | "warning" | "danger"> = {
  low: "outline",
  normal: "default",
  high: "warning",
  urgent: "danger",
};

export interface QueueItemCardProps {
  publication: Publication;
  channelName: string;
  timezone: string;
  dense?: boolean;
  onClick?: () => void;
  /** Section 72-76: surfaced, never queue-cancelling — omit the prop (or
   * pass `true`) when the target platform has no connected-account
   * concept wired yet, so existing callers keep their current behavior. */
  accountConnected?: boolean;
}

export function QueueItemCard({
  publication,
  channelName,
  timezone,
  dense = false,
  onClick,
  accountConnected = true,
}: QueueItemCardProps) {
  const progress = usePublicationProgress(publication.id);
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex w-full items-center gap-3 rounded-lg border border-border bg-surface p-3 text-left transition-colors hover:border-border-strong",
        dense && "p-2.5",
      )}
    >
      <MediaThumbnail seed={publication.video_id} className={dense ? "h-11 w-8" : "h-14 w-10"} />

      <div className="min-w-0 flex-1">
        <p className="truncate text-body-small font-medium text-foreground">{publication.title}</p>
        <div className="mt-0.5 flex items-center gap-2">
          <p className="text-caption normal-case tracking-normal">{channelName}</p>
          <Badge variant={PRIORITY_VARIANT[publication.priority]} className="px-1.5 py-0 text-[0.6875rem]">
            {VIDEO_PRIORITY_LABELS[publication.priority]}
          </Badge>
          {publication.locked && <Lock className="size-3 text-muted-foreground" aria-label="Locked" />}
        </div>
      </div>

      <div className="flex items-center gap-1.5">
        {!accountConnected && (
          <Tooltip>
            <TooltipTrigger asChild>
              <TriangleAlert
                className="size-3.5 text-warning"
                aria-label="No connected account for this platform"
              />
            </TooltipTrigger>
            <TooltipContent>
              No connected {publication.platform} account for this channel — it can stay queued, but
              publishing will be blocked until one is connected.
            </TooltipContent>
          </Tooltip>
        )}
        <PlatformBadge platform={publication.platform} size="sm" iconOnly />
        <StatusBadge status={publication.status} className="px-1.5 py-0 text-[0.625rem]" />
        {publication.status === "uploading" && progress?.percentage != null && (
          <span className="font-mono-data text-[0.625rem] text-primary" aria-label="Upload progress">
            {Math.round(progress.percentage)}%
          </span>
        )}
      </div>

      <span className="w-16 shrink-0 text-right font-mono-data text-body-small text-muted-foreground">
        {publication.scheduled_at ? formatTimeInZone(publication.scheduled_at, timezone) : "—"}
      </span>
    </button>
  );
}
