import { MediaThumbnail } from "@/components/common/media-thumbnail";
import { StatusBadge } from "@/components/ui/status-badge";
import type { MockContentItem } from "@/development/mock-data/content";
import { mockContentChannelName } from "@/development/mock-data/content";
import { formatBytes, formatDuration } from "@/lib/formatting/number";

const SOURCE_LABEL: Record<MockContentItem["source"], string> = {
  local_file: "Local file",
  cut_pro_export: "Cut.pro export",
  manual_upload: "Manual upload",
};

export function ContentListRow({ item }: { item: MockContentItem }) {
  return (
    <div className="grid grid-cols-[3rem_1fr_6rem_8rem_7rem_6rem_6rem] items-center gap-3 rounded-md px-2 py-2 transition-colors hover:bg-surface-hover">
      <MediaThumbnail seed={item.id} className="h-11 w-8" />
      <span className="truncate text-body-small font-medium text-foreground">{item.title}</span>
      <span className="font-mono-data text-body-small text-muted-foreground">
        {formatDuration(item.durationSeconds)}
      </span>
      <span className="truncate text-body-small text-muted-foreground">
        {mockContentChannelName(item.channelId)}
      </span>
      <span className="text-body-small text-muted-foreground">{SOURCE_LABEL[item.source]}</span>
      <StatusBadge status={item.status} className="w-fit px-1.5 py-0 text-[0.625rem]" />
      <span className="text-right font-mono-data text-body-small text-muted-foreground">
        {formatBytes(item.fileSizeBytes)}
      </span>
    </div>
  );
}

export function ContentListHeader() {
  return (
    <div className="grid grid-cols-[3rem_1fr_6rem_8rem_7rem_6rem_6rem] gap-3 px-2 pb-1 text-caption">
      <span />
      <span>Title</span>
      <span>Duration</span>
      <span>Channel</span>
      <span>Source</span>
      <span>Status</span>
      <span className="text-right">Size</span>
    </div>
  );
}
