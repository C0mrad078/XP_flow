import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { schedulerApi } from "@/lib/tauri";
import { useWorkspaceStore } from "@/stores/workspace-store";
import type { ISODateTime, UUID } from "@/types/domain";
import type { CalendarPublication } from "@/types/scheduling";

function useWorkspaceId(): string | undefined {
  return useWorkspaceStore((state) => state.workspace?.id);
}

function useInvalidateSchedule() {
  const queryClient = useQueryClient();
  return () => {
    queryClient.invalidateQueries({ queryKey: ["queue"] });
    queryClient.invalidateQueries({ queryKey: ["calendar"] });
    queryClient.invalidateQueries({ queryKey: ["due-publications"] });
  };
}

export function useCalendarRange(channelId: UUID | null, start: string | null, end: string | null) {
  const workspaceId = useWorkspaceId();
  return useQuery({
    queryKey: ["calendar", workspaceId, channelId, start, end],
    queryFn: () => schedulerApi.calendarRange(workspaceId!, channelId, start!, end!),
    enabled: Boolean(workspaceId && start && end),
  });
}

export function useDuePublications() {
  const workspaceId = useWorkspaceId();
  return useQuery({
    queryKey: ["due-publications", workspaceId],
    queryFn: () => schedulerApi.listDue(workspaceId!),
    enabled: Boolean(workspaceId),
    refetchInterval: 60_000,
  });
}

export function useSchedulePublication() {
  const invalidate = useInvalidateSchedule();
  return useMutation({
    mutationFn: ({ publicationId, at }: { publicationId: UUID; at: ISODateTime }) =>
      schedulerApi.scheduleAt(publicationId, at),
    onSuccess: invalidate,
  });
}

export function useUnschedulePublication() {
  const invalidate = useInvalidateSchedule();
  return useMutation({
    mutationFn: (publicationId: UUID) => schedulerApi.unschedule(publicationId),
    onSuccess: invalidate,
  });
}

/** Calendar drag-and-drop reschedule (section 42-45): applies the move to
 * every cached calendar query optimistically so the chip jumps to the
 * target day immediately, rolls every one of them back if the backend
 * rejects the move (locked publication, paused channel, slot conflict),
 * and always reconciles with the server afterwards in case the exact
 * local time shifted (e.g. a DST edge). */
export function useReschedulePublicationToDate() {
  const queryClient = useQueryClient();
  const invalidate = useInvalidateSchedule();

  return useMutation({
    mutationFn: ({ publicationId, newDate }: { publicationId: UUID; newDate: string }) =>
      schedulerApi.rescheduleToDate(publicationId, newDate),
    onMutate: async ({ publicationId, newDate }) => {
      await queryClient.cancelQueries({ queryKey: ["calendar"] });
      const previous = queryClient.getQueriesData<CalendarPublication[]>({ queryKey: ["calendar"] });
      queryClient.setQueriesData<CalendarPublication[]>({ queryKey: ["calendar"] }, (items) =>
        items?.map((item) =>
          item.publication.id === publicationId ? { ...item, local_date: newDate } : item,
        ),
      );
      return { previous };
    },
    onError: (_error, _variables, context) => {
      context?.previous.forEach(([key, data]) => queryClient.setQueryData(key, data));
    },
    onSettled: invalidate,
  });
}

export function useAutoSchedulePublication() {
  const invalidate = useInvalidateSchedule();
  return useMutation({
    mutationFn: (publicationId: UUID) => schedulerApi.autoSchedule(publicationId),
    onSuccess: invalidate,
  });
}

export function useAutoScheduleChannel() {
  const invalidate = useInvalidateSchedule();
  return useMutation({
    mutationFn: ({ channelId, horizonDays }: { channelId: UUID; horizonDays?: number }) =>
      schedulerApi.autoScheduleChannel(channelId, horizonDays),
    onSuccess: invalidate,
  });
}

export function useFillScheduleGaps() {
  const invalidate = useInvalidateSchedule();
  return useMutation({
    mutationFn: (channelId: UUID) => schedulerApi.fillScheduleGaps(channelId),
    onSuccess: invalidate,
  });
}

export function useRebuildChannelSchedule() {
  const invalidate = useInvalidateSchedule();
  return useMutation({
    mutationFn: ({ channelId, horizonDays }: { channelId: UUID; horizonDays?: number }) =>
      schedulerApi.rebuildChannelSchedule(channelId, horizonDays),
    onSuccess: invalidate,
  });
}
