import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { scheduleSlotsApi } from "@/lib/tauri";
import type { Platform, UUID } from "@/types/domain";

function useInvalidateSlots(channelId: UUID) {
  const queryClient = useQueryClient();
  return () => {
    queryClient.invalidateQueries({ queryKey: ["schedule-slots", channelId] });
    queryClient.invalidateQueries({ queryKey: ["calendar"] });
  };
}

export function useScheduleSlots(channelId: UUID | null) {
  return useQuery({
    queryKey: ["schedule-slots", channelId],
    queryFn: () => scheduleSlotsApi.list(channelId!),
    enabled: Boolean(channelId),
  });
}

export function useCreateScheduleSlot(channelId: UUID) {
  const invalidate = useInvalidateSlots(channelId);
  return useMutation({
    mutationFn: ({
      platform,
      dayOfWeek,
      timeOfDay,
    }: {
      platform: Platform | null;
      dayOfWeek: number;
      timeOfDay: string;
    }) => scheduleSlotsApi.create(channelId, platform, dayOfWeek, timeOfDay),
    onSuccess: invalidate,
  });
}

export function useSetScheduleSlotActive(channelId: UUID) {
  const invalidate = useInvalidateSlots(channelId);
  return useMutation({
    mutationFn: ({ id, isActive }: { id: UUID; isActive: boolean }) =>
      scheduleSlotsApi.setActive(id, isActive),
    onSuccess: invalidate,
  });
}

export function useDeleteScheduleSlot(channelId: UUID) {
  const invalidate = useInvalidateSlots(channelId);
  return useMutation({
    mutationFn: (id: UUID) => scheduleSlotsApi.remove(id),
    onSuccess: invalidate,
  });
}

export function useCopyScheduleDay(channelId: UUID) {
  const invalidate = useInvalidateSlots(channelId);
  return useMutation({
    mutationFn: ({ fromDay, toDays }: { fromDay: number; toDays: number[] }) =>
      scheduleSlotsApi.copyDay(channelId, fromDay, toDays),
    onSuccess: invalidate,
  });
}

export function useScheduleExceptions(channelId: UUID | null, from: string | null, to: string | null) {
  return useQuery({
    queryKey: ["schedule-exceptions", channelId, from, to],
    queryFn: () => scheduleSlotsApi.listExceptions(channelId!, from!, to!),
    enabled: Boolean(channelId && from && to),
  });
}

export function useAddScheduleException(channelId: UUID) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ date, reason }: { date: string; reason: string | null }) =>
      scheduleSlotsApi.addException(channelId, date, reason),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["schedule-exceptions", channelId] });
      queryClient.invalidateQueries({ queryKey: ["calendar"] });
    },
  });
}

export function useRemoveScheduleException(channelId: UUID) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: UUID) => scheduleSlotsApi.removeException(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["schedule-exceptions", channelId] });
      queryClient.invalidateQueries({ queryKey: ["calendar"] });
    },
  });
}
