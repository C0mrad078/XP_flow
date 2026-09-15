import { QueueItemCard } from "./queue-item-card";
import type { Publication, UUID } from "@/types/domain";

export interface QueueListViewProps {
  publications: Publication[];
  channelNames: Map<UUID, string>;
  timezone: string;
  onSelect?: (publication: Publication) => void;
}

export function QueueListView({ publications, channelNames, timezone, onSelect }: QueueListViewProps) {
  return (
    <div className="flex flex-col gap-2">
      {publications.map((publication) => (
        <QueueItemCard
          key={publication.id}
          publication={publication}
          channelName={channelNames.get(publication.channel_id) ?? "Unknown channel"}
          timezone={timezone}
          dense
          onClick={onSelect ? () => onSelect(publication) : undefined}
        />
      ))}
    </div>
  );
}
