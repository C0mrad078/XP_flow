import { useState } from "react";
import { Copy, Plus, Trash2 } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Divider } from "@/components/ui/divider";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import {
  useAddScheduleException,
  useCopyScheduleDay,
  useCreateScheduleSlot,
  useDeleteScheduleSlot,
  useRemoveScheduleException,
  useScheduleExceptions,
  useScheduleSlots,
  useSetScheduleSlotActive,
} from "@/hooks/use-schedule-slots";
import { toast } from "@/stores/toast-store";
import { PLATFORM_LABELS, PLATFORMS, isAppError, type Platform, type UUID } from "@/types/domain";

const WEEKDAYS = [
  { index: 0, label: "Monday" },
  { index: 1, label: "Tuesday" },
  { index: 2, label: "Wednesday" },
  { index: 3, label: "Thursday" },
  { index: 4, label: "Friday" },
  { index: 5, label: "Saturday" },
  { index: 6, label: "Sunday" },
];

const ALL_PLATFORMS_VALUE = "__all__";

function todayIsoDate(): string {
  return new Date().toISOString().slice(0, 10);
}

function addDaysIso(days: number): string {
  const d = new Date();
  d.setDate(d.getDate() + days);
  return d.toISOString().slice(0, 10);
}

export interface ChannelScheduleDrawerProps {
  channelId: UUID | null;
  channelName: string;
  onClose: () => void;
}

export function ChannelScheduleDrawer({ channelId, channelName, onClose }: ChannelScheduleDrawerProps) {
  return (
    <Sheet open={Boolean(channelId)} onOpenChange={(open) => !open && onClose()}>
      <SheetContent side="right" className="w-full max-w-lg overflow-y-auto">
        <SheetHeader>
          <SheetTitle>{channelName} — Weekly schedule</SheetTitle>
        </SheetHeader>
        {channelId && <ScheduleBody key={channelId} channelId={channelId} />}
      </SheetContent>
    </Sheet>
  );
}

function ScheduleBody({ channelId }: { channelId: UUID }) {
  const { data: slots = [] } = useScheduleSlots(channelId);
  const { data: exceptions = [] } = useScheduleExceptions(channelId, todayIsoDate(), addDaysIso(60));
  const createSlot = useCreateScheduleSlot(channelId);
  const setActive = useSetScheduleSlotActive(channelId);
  const deleteSlot = useDeleteScheduleSlot(channelId);
  const copyDay = useCopyScheduleDay(channelId);
  const addException = useAddScheduleException(channelId);
  const removeException = useRemoveScheduleException(channelId);

  const [addingForDay, setAddingForDay] = useState<number | null>(null);
  const [draftTime, setDraftTime] = useState("18:00");
  const [draftPlatform, setDraftPlatform] = useState<string>(ALL_PLATFORMS_VALUE);
  const [exceptionDate, setExceptionDate] = useState(addDaysIso(1));

  const activeSlotCount = slots.filter((s) => s.is_active).length;

  function handleAddSlot(day: number) {
    const platform = draftPlatform === ALL_PLATFORMS_VALUE ? null : (draftPlatform as Platform);
    createSlot.mutate(
      { platform, dayOfWeek: day, timeOfDay: draftTime },
      {
        onSuccess: () => setAddingForDay(null),
        onError: (error) =>
          toast({
            variant: "error",
            title: "Couldn't add slot",
            description: isAppError(error) ? error.user_message : "A slot may already exist at that time.",
          }),
      },
    );
  }

  function handleCopyDay(day: number) {
    const targets = WEEKDAYS.filter((w) => w.index !== day).map((w) => w.index);
    copyDay.mutate(
      { fromDay: day, toDays: targets },
      {
        onSuccess: (created) =>
          toast({
            variant: "success",
            title: `Copied to every other day`,
            description: `${created.length} slot(s) created`,
          }),
      },
    );
  }

  return (
    <div className="flex flex-col gap-5">
      <div className="rounded-lg border border-border bg-surface-elevated p-3">
        <p className="text-body-small text-muted-foreground">
          <strong className="text-foreground">{activeSlotCount}</strong> active slot(s) — every 7-day window
          covers each weekday once, so this is also the next-7-days publishing capacity (skip exceptions not
          included).
        </p>
      </div>

      <div className="flex flex-col gap-4">
        {WEEKDAYS.map(({ index, label }) => {
          const daySlots = slots
            .filter((s) => s.day_of_week === index)
            .sort((a, b) => a.time_of_day.localeCompare(b.time_of_day));

          return (
            <div key={index} className="flex flex-col gap-2">
              <div className="flex items-center justify-between">
                <span className="text-body-small font-medium text-foreground">{label}</span>
                <div className="flex items-center gap-1">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => handleCopyDay(index)}
                    title="Copy this day's slots to every other day"
                  >
                    <Copy className="size-3.5" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setAddingForDay(addingForDay === index ? null : index)}
                  >
                    <Plus className="size-3.5" />
                  </Button>
                </div>
              </div>

              <div className="flex flex-wrap gap-1.5">
                {daySlots.length === 0 && (
                  <span className="text-caption normal-case tracking-normal">No slots</span>
                )}
                {daySlots.map((slot) => (
                  <button
                    key={slot.id}
                    type="button"
                    onClick={() => setActive.mutate({ id: slot.id, isActive: !slot.is_active })}
                    className="group flex items-center gap-1 rounded-full border border-border bg-surface px-2 py-1 text-caption normal-case tracking-normal"
                  >
                    <span className={slot.is_active ? "text-foreground" : "text-muted"}>
                      {slot.time_of_day} {slot.platform ? `· ${PLATFORM_LABELS[slot.platform]}` : "· All"}
                    </span>
                    {!slot.is_active && (
                      <Badge variant="outline" className="px-1 py-0 text-[0.625rem]">
                        Paused
                      </Badge>
                    )}
                    <Trash2
                      className="size-3 text-muted opacity-0 group-hover:opacity-100"
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteSlot.mutate(slot.id);
                      }}
                    />
                  </button>
                ))}
              </div>

              {addingForDay === index && (
                <div className="flex items-center gap-2 rounded-md border border-border bg-surface p-2">
                  <Input
                    type="time"
                    value={draftTime}
                    onChange={(e) => setDraftTime(e.target.value)}
                    className="h-8 w-28"
                  />
                  <Select value={draftPlatform} onValueChange={setDraftPlatform}>
                    <SelectTrigger className="h-8 w-36">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value={ALL_PLATFORMS_VALUE}>All platforms</SelectItem>
                      {PLATFORMS.map((p) => (
                        <SelectItem key={p} value={p}>
                          {PLATFORM_LABELS[p]}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <Button size="sm" onClick={() => handleAddSlot(index)} disabled={createSlot.isPending}>
                    Add
                  </Button>
                </div>
              )}
            </div>
          );
        })}
      </div>

      <Divider />

      <div className="flex flex-col gap-2">
        <span className="text-body-small font-medium text-foreground">Skip dates</span>
        <div className="flex items-center gap-2">
          <Input
            type="date"
            value={exceptionDate}
            onChange={(e) => setExceptionDate(e.target.value)}
            className="h-8"
          />
          <Button
            size="sm"
            variant="outline"
            onClick={() =>
              addException.mutate(
                { date: exceptionDate, reason: null },
                {
                  onError: (error) =>
                    toast({
                      variant: "error",
                      title: "Couldn't add skip date",
                      description: isAppError(error) ? error.user_message : undefined,
                    }),
                },
              )
            }
          >
            Skip this date
          </Button>
        </div>
        <div className="flex flex-col gap-1.5">
          {exceptions.map((exception) => (
            <div key={exception.id} className="flex items-center justify-between text-body-small">
              <span className="text-foreground">{exception.date}</span>
              <Button variant="ghost" size="sm" onClick={() => removeException.mutate(exception.id)}>
                <Trash2 className="size-3.5" />
              </Button>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
