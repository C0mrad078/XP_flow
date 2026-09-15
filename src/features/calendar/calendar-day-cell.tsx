import { useDroppable } from "@dnd-kit/core";

import { toDateKey } from "@/features/calendar/calendar-dates";
import { CalendarPublicationChip } from "@/features/calendar/calendar-publication-chip";
import { cn } from "@/lib/utilities/cn";
import type { CalendarPublication } from "@/types/scheduling";

export interface CalendarDayCellProps {
  date: Date;
  items: CalendarPublication[];
  channelNames: Map<string, string>;
  inCurrentMonth?: boolean;
  isToday?: boolean;
  compact?: boolean;
  onSelectPublication: (id: string) => void;
}

export function CalendarDayCell({
  date,
  items,
  channelNames,
  inCurrentMonth = true,
  isToday = false,
  compact = false,
  onSelectPublication,
}: CalendarDayCellProps) {
  const dateKey = toDateKey(date);
  const { setNodeRef, isOver } = useDroppable({ id: dateKey, data: { dateKey } });

  const sorted = [...items].sort((a, b) => a.local_time.localeCompare(b.local_time));
  const visible = compact ? sorted.slice(0, 3) : sorted;
  const overflow = compact ? sorted.length - visible.length : 0;

  return (
    <div
      ref={setNodeRef}
      className={cn(
        "flex min-h-24 flex-col gap-1 border-b border-r border-border p-1.5",
        !inCurrentMonth && "bg-surface-elevated/40",
        isOver && "bg-primary/5 ring-1 ring-inset ring-primary/40",
      )}
    >
      <span
        className={cn(
          "text-caption normal-case tracking-normal",
          !inCurrentMonth && "text-muted",
          isToday && "font-semibold text-primary",
        )}
      >
        {date.getDate()}
      </span>
      <div className="flex flex-col gap-1">
        {visible.map((item) => (
          <CalendarPublicationChip
            key={item.publication.id}
            item={item}
            channelName={channelNames.get(item.publication.channel_id) ?? "Unknown channel"}
            onClick={() => onSelectPublication(item.publication.id)}
          />
        ))}
        {overflow > 0 && <span className="text-caption normal-case tracking-normal">+{overflow} more</span>}
      </div>
    </div>
  );
}
