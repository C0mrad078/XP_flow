import { useDraggable } from "@dnd-kit/core";
import { CSS } from "@dnd-kit/utilities";
import { Lock } from "lucide-react";

import { PlatformBadge } from "@/components/ui/platform-badge";
import { cn } from "@/lib/utilities/cn";
import type { CalendarPublication } from "@/types/scheduling";

export interface CalendarPublicationChipProps {
  item: CalendarPublication;
  channelName: string;
  onClick: () => void;
}

export function CalendarPublicationChip({ item, channelName, onClick }: CalendarPublicationChipProps) {
  const { publication } = item;
  const locked = publication.locked;
  const { attributes, listeners, setNodeRef, transform, isDragging } = useDraggable({
    id: publication.id,
    data: { publication },
    disabled: locked,
  });

  return (
    <button
      ref={setNodeRef}
      type="button"
      onClick={onClick}
      {...listeners}
      {...attributes}
      style={{ transform: CSS.Translate.toString(transform) }}
      title={locked ? `${publication.title} (locked — drag to reschedule disabled)` : publication.title}
      className={cn(
        "flex w-full items-center gap-1 rounded-md border border-border bg-surface px-1.5 py-1 text-left text-[0.6875rem] leading-tight",
        "hover:border-border-strong",
        isDragging && "opacity-50",
        locked ? "cursor-default" : "cursor-grab active:cursor-grabbing",
      )}
    >
      <PlatformBadge platform={publication.platform} size="sm" iconOnly />
      <span className="min-w-0 flex-1 truncate text-foreground">
        <span className="font-mono-data text-muted-foreground">{item.local_time}</span> {publication.title}
      </span>
      {locked && <Lock className="size-2.5 shrink-0 text-muted-foreground" />}
      <span className="sr-only">{channelName}</span>
    </button>
  );
}
