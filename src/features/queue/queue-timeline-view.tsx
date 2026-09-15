import { QueueItemCard } from "./queue-item-card";
import { formatDateInZone, localDateKeyInZone } from "@/lib/formatting/date";
import type { Publication, UUID } from "@/types/domain";

export interface QueueTimelineViewProps {
  publications: Publication[];
  channelNames: Map<UUID, string>;
  timezone: string;
  onSelect?: (publication: Publication) => void;
  isAccountConnected?: (publication: Publication) => boolean;
}

const UNSCHEDULED_LABEL = "Unscheduled";

function dayLabel(iso: string, timezone: string): string {
  const key = localDateKeyInZone(iso, timezone);
  const now = new Date();
  const todayKey = localDateKeyInZone(now.toISOString(), timezone);
  const tomorrow = new Date(now);
  tomorrow.setUTCDate(tomorrow.getUTCDate() + 1);
  const tomorrowKey = localDateKeyInZone(tomorrow.toISOString(), timezone);

  if (key === todayKey) return "Today";
  if (key === tomorrowKey) return "Tomorrow";
  return formatDateInZone(iso, timezone);
}

export function QueueTimelineView({
  publications,
  channelNames,
  timezone,
  onSelect,
  isAccountConnected,
}: QueueTimelineViewProps) {
  const groups = new Map<string, Publication[]>();
  const unscheduled = publications.filter((p) => !p.scheduled_at);
  if (unscheduled.length > 0) groups.set(UNSCHEDULED_LABEL, unscheduled);

  const scheduled = [...publications]
    .filter((p): p is Publication & { scheduled_at: string } => p.scheduled_at !== null)
    .sort((a, b) => new Date(a.scheduled_at).getTime() - new Date(b.scheduled_at).getTime());

  for (const publication of scheduled) {
    const label = dayLabel(publication.scheduled_at, timezone);
    const bucket = groups.get(label) ?? [];
    bucket.push(publication);
    groups.set(label, bucket);
  }

  return (
    <div className="flex flex-col gap-6">
      {Array.from(groups.entries()).map(([label, groupItems]) => (
        <div key={label} className="flex gap-4">
          <div className="flex w-20 shrink-0 flex-col items-end pt-1">
            <span className="text-label text-foreground">{label}</span>
            <span className="text-caption">{groupItems.length} posts</span>
          </div>
          <div className="relative flex-1 border-l border-border pl-5">
            <div className="flex flex-col gap-2.5">
              {groupItems.map((publication) => (
                <div key={publication.id} className="relative">
                  <span className="absolute -left-[25px] top-1/2 size-2 -translate-y-1/2 rounded-full border-2 border-background bg-primary" />
                  <QueueItemCard
                    publication={publication}
                    channelName={channelNames.get(publication.channel_id) ?? "Unknown channel"}
                    timezone={timezone}
                    onClick={onSelect ? () => onSelect(publication) : undefined}
                    accountConnected={isAccountConnected ? isAccountConnected(publication) : true}
                  />
                </div>
              ))}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}
