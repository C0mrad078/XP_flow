import { MediaThumbnail } from "@/components/common/media-thumbnail";
import { Badge } from "@/components/ui/badge";
import { PlatformBadge } from "@/components/ui/platform-badge";
import { StatusBadge } from "@/components/ui/status-badge";
import type { MockQueueItem, QueuePriority } from "@/development/mock-data/queue";
import { formatTime } from "@/lib/formatting/date";
import { cn } from "@/lib/utilities/cn";
import type { Platform } from "@/types/domain";

const PRIORITY_LABEL: Record<QueuePriority, string> = { low: "Low", normal: "Normal", high: "High" };
const PRIORITY_VARIANT: Record<QueuePriority, "outline" | "default" | "warning"> = {
  low: "outline",
  normal: "default",
  high: "warning",
};

export interface QueueItemCardProps {
  item: MockQueueItem;
  dense?: boolean;
}

export function QueueItemCard({ item, dense = false }: QueueItemCardProps) {
  const platforms = Object.entries(item.platforms) as [Platform, MockQueueItem["platforms"][Platform]][];

  return (
    <div
      className={cn(
        "flex items-center gap-3 rounded-lg border border-border bg-surface p-3 transition-colors hover:border-border-strong",
        dense && "p-2.5",
      )}
    >
      <MediaThumbnail
        seed={item.id}
        durationSeconds={item.durationSeconds}
        className={dense ? "h-11 w-8" : "h-14 w-10"}
      />

      <div className="min-w-0 flex-1">
        <p className="truncate text-body-small font-medium text-foreground">{item.videoTitle}</p>
        <div className="mt-0.5 flex items-center gap-2">
          <p className="text-caption normal-case tracking-normal">{item.channelName}</p>
          <Badge variant={PRIORITY_VARIANT[item.priority]} className="px-1.5 py-0 text-[0.6875rem]">
            {PRIORITY_LABEL[item.priority]}
          </Badge>
        </div>
      </div>

      <div className="flex items-center gap-1.5">
        {platforms.map(([platform, status]) =>
          status ? (
            <div key={platform} className="flex flex-col items-center gap-1">
              <PlatformBadge platform={platform} size="sm" iconOnly />
              <StatusBadge status={status} className="px-1.5 py-0 text-[0.625rem]" />
            </div>
          ) : null,
        )}
      </div>

      <span className="w-16 shrink-0 text-right font-mono-data text-body-small text-muted-foreground">
        {formatTime(item.scheduledAt)}
      </span>
    </div>
  );
}
