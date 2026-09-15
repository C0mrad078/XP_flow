import { MediaThumbnail } from "@/components/common/media-thumbnail";
import { StatusBadge } from "@/components/ui/status-badge";
import type { MockContentItem } from "@/development/mock-data/content";
import { mockContentChannelName } from "@/development/mock-data/content";
import { formatDate } from "@/lib/formatting/date";

const SOURCE_LABEL: Record<MockContentItem["source"], string> = {
  local_file: "Local file",
  cut_pro_export: "Cut.pro export",
  manual_upload: "Manual upload",
};

export function ContentItemCard({ item }: { item: MockContentItem }) {
  return (
    <div className="group flex flex-col gap-2.5 rounded-lg border border-border bg-surface p-2.5 transition-colors hover:border-border-strong">
      <MediaThumbnail seed={item.id} durationSeconds={item.durationSeconds} className="w-full" />
      <div className="flex flex-col gap-1 px-0.5">
        <p className="truncate text-body-small font-medium text-foreground">{item.title}</p>
        <p className="text-caption normal-case tracking-normal">{mockContentChannelName(item.channelId)}</p>
        <div className="flex items-center justify-between pt-1">
          <StatusBadge status={item.status} className="px-1.5 py-0 text-[0.625rem]" />
          <span className="text-caption normal-case tracking-normal">{SOURCE_LABEL[item.source]}</span>
        </div>
        <p className="text-caption normal-case tracking-normal">Imported {formatDate(item.importedAt)}</p>
      </div>
    </div>
  );
}
