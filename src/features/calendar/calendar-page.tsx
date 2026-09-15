import { useState } from "react";
import { DndContext, type DragEndEvent } from "@dnd-kit/core";
import { ChevronLeft, ChevronRight } from "lucide-react";

import { PageContainer } from "@/components/common/page-container";
import { PageHeader } from "@/components/common/page-header";
import { ErrorState } from "@/components/feedback/error-state";
import { LoadingState } from "@/components/feedback/loading-state";
import { Button } from "@/components/ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useChannels } from "@/hooks/use-channels";
import { useCalendarRange, useReschedulePublicationToDate } from "@/hooks/use-scheduler";
import { toast } from "@/stores/toast-store";
import { useWorkspaceStore } from "@/stores/workspace-store";
import { isAppError } from "@/types/domain";

import { isSameDay, isSameMonth, monthGridDates, startOfWeek, toDateKey, weekDates } from "./calendar-dates";
import { CalendarDayCell } from "./calendar-day-cell";
import { PublicationDetailsDrawer } from "../queue/publication-details-drawer";

type CalendarViewMode = "month" | "week";

const WEEKDAY_LABELS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

export function CalendarPage() {
  const [viewMode, setViewMode] = useState<CalendarViewMode>("month");
  const [anchor, setAnchor] = useState(() => new Date());
  const [channelId, setChannelId] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const workspaceId = useWorkspaceStore((state) => state.workspace?.id ?? null);
  const { data: channels = [] } = useChannels();
  const channelNames = new Map(channels.map((c) => [c.id, c.name]));

  const gridDates = viewMode === "month" ? monthGridDates(anchor) : weekDates(anchor);
  const firstGridDate = gridDates[0]!;
  const lastGridDate = gridDates[gridDates.length - 1]!;
  const start = toDateKey(firstGridDate);
  const end = toDateKey(lastGridDate);

  const { data: items = [], isLoading, isError, refetch } = useCalendarRange(channelId, start, end);
  const reschedule = useReschedulePublicationToDate();

  const itemsByDate = new Map<string, typeof items>();
  for (const item of items) {
    const bucket = itemsByDate.get(item.local_date) ?? [];
    bucket.push(item);
    itemsByDate.set(item.local_date, bucket);
  }

  function step(direction: 1 | -1) {
    const next = new Date(anchor);
    if (viewMode === "month") {
      next.setMonth(next.getMonth() + direction);
    } else {
      next.setDate(next.getDate() + direction * 7);
    }
    setAnchor(next);
  }

  function handleDragEnd(event: DragEndEvent) {
    const publicationId = event.active.id as string;
    const targetDate = event.over?.id as string | undefined;
    if (!targetDate) return;

    const item = items.find((i) => i.publication.id === publicationId);
    if (!item || item.local_date === targetDate) return;

    reschedule.mutate(
      { publicationId, newDate: targetDate },
      {
        onError: (error) =>
          toast({
            variant: "error",
            title: "Couldn't reschedule",
            description: isAppError(error) ? error.user_message : undefined,
          }),
      },
    );
  }

  const today = new Date();
  const rangeLabel =
    viewMode === "month"
      ? anchor.toLocaleDateString(undefined, { month: "long", year: "numeric" })
      : `${startOfWeek(anchor).toLocaleDateString(undefined, { month: "short", day: "numeric" })} – ${lastGridDate.toLocaleDateString(undefined, { month: "short", day: "numeric" })}`;

  return (
    <PageContainer>
      <PageHeader
        title="Calendar"
        description="Every scheduled publication across channels and platforms. Drag a chip to a new day to reschedule it."
      />

      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <Button variant="ghost" size="icon" onClick={() => step(-1)} aria-label="Previous">
            <ChevronLeft className="size-4" />
          </Button>
          <span className="w-40 text-center text-body-small font-medium text-foreground">{rangeLabel}</span>
          <Button variant="ghost" size="icon" onClick={() => step(1)} aria-label="Next">
            <ChevronRight className="size-4" />
          </Button>
          <Button variant="outline" size="sm" onClick={() => setAnchor(new Date())}>
            Today
          </Button>
        </div>

        <div className="flex items-center gap-2">
          <Select
            value={channelId ?? "__all__"}
            onValueChange={(v) => setChannelId(v === "__all__" ? null : v)}
          >
            <SelectTrigger className="h-9 w-44">
              <SelectValue placeholder="All channels" />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="__all__">All channels</SelectItem>
              {channels.map((c) => (
                <SelectItem key={c.id} value={c.id}>
                  {c.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <Tabs value={viewMode} onValueChange={(v) => setViewMode(v as CalendarViewMode)}>
            <TabsList>
              <TabsTrigger value="month">Month</TabsTrigger>
              <TabsTrigger value="week">Week</TabsTrigger>
            </TabsList>
          </Tabs>
        </div>
      </div>

      {isLoading && <LoadingState label="Loading calendar…" />}
      {isError && <ErrorState onRetry={() => refetch()} />}

      {!isLoading && !isError && workspaceId && (
        <DndContext onDragEnd={handleDragEnd}>
          <div className="overflow-hidden rounded-lg border-l border-t border-border">
            <div className="grid grid-cols-7 border-b border-border bg-surface-elevated">
              {WEEKDAY_LABELS.map((label) => (
                <div
                  key={label}
                  className="border-r border-border px-2 py-1.5 text-caption normal-case tracking-normal"
                >
                  {label}
                </div>
              ))}
            </div>
            <div className="grid grid-cols-7">
              {gridDates.map((date) => (
                <CalendarDayCell
                  key={toDateKey(date)}
                  date={date}
                  items={itemsByDate.get(toDateKey(date)) ?? []}
                  channelNames={channelNames}
                  inCurrentMonth={viewMode === "week" || isSameMonth(date, anchor)}
                  isToday={isSameDay(date, today)}
                  compact={viewMode === "month"}
                  onSelectPublication={setSelectedId}
                />
              ))}
            </div>
          </div>
        </DndContext>
      )}

      <PublicationDetailsDrawer publicationId={selectedId} onClose={() => setSelectedId(null)} />
    </PageContainer>
  );
}
