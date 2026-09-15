import { QueueItemCard } from "./queue-item-card";
import type { MockQueueItem } from "@/development/mock-data/queue";
import { formatDate } from "@/lib/formatting/date";

function dayLabel(iso: string): string {
  const date = new Date(iso);
  const today = new Date();
  const tomorrow = new Date(today);
  tomorrow.setDate(today.getDate() + 1);

  if (isSameDay(date, today)) return "Today";
  if (isSameDay(date, tomorrow)) return "Tomorrow";
  return formatDate(iso);
}

function isSameDay(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

export function QueueTimelineView({ items }: { items: MockQueueItem[] }) {
  const sorted = [...items].sort(
    (a, b) => new Date(a.scheduledAt).getTime() - new Date(b.scheduledAt).getTime(),
  );

  const groups = new Map<string, MockQueueItem[]>();
  for (const item of sorted) {
    const label = dayLabel(item.scheduledAt);
    const bucket = groups.get(label) ?? [];
    bucket.push(item);
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
              {groupItems.map((item) => (
                <div key={item.id} className="relative">
                  <span className="absolute -left-[25px] top-1/2 size-2 -translate-y-1/2 rounded-full border-2 border-background bg-primary" />
                  <QueueItemCard item={item} />
                </div>
              ))}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}
