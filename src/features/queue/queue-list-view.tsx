import { QueueItemCard } from "./queue-item-card";
import type { MockQueueItem } from "@/development/mock-data/queue";

export function QueueListView({ items }: { items: MockQueueItem[] }) {
  const sorted = [...items].sort(
    (a, b) => new Date(a.scheduledAt).getTime() - new Date(b.scheduledAt).getTime(),
  );

  return (
    <div className="flex flex-col gap-2">
      {sorted.map((item) => (
        <QueueItemCard key={item.id} item={item} dense />
      ))}
    </div>
  );
}
