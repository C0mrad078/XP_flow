import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { queueApi } from "@/lib/tauri";
import { useWorkspaceStore } from "@/stores/workspace-store";
import type { Platform, UUID } from "@/types/domain";
import type { VideoPriority } from "@/types/media";
import type { PublicationListRequest } from "@/types/scheduling";

function useWorkspaceId(): string | undefined {
  return useWorkspaceStore((state) => state.workspace?.id);
}

export function useQueueList(request: PublicationListRequest) {
  const workspaceId = useWorkspaceId();
  return useQuery({
    queryKey: ["queue", workspaceId, request],
    queryFn: () => queueApi.list(workspaceId!, request),
    enabled: Boolean(workspaceId),
    placeholderData: (previous) => previous,
  });
}

export function usePublication(id: UUID | null) {
  return useQuery({
    queryKey: ["publication", id],
    queryFn: () => queueApi.get(id!),
    enabled: Boolean(id),
  });
}

function useInvalidateQueue() {
  const queryClient = useQueryClient();
  return () => {
    queryClient.invalidateQueries({ queryKey: ["queue"] });
    queryClient.invalidateQueries({ queryKey: ["calendar"] });
  };
}

export function useAddToQueue() {
  const workspaceId = useWorkspaceId();
  const invalidate = useInvalidateQueue();
  return useMutation({
    mutationFn: ({
      videoId,
      channelId,
      platform,
      platformAccountId = null,
      priority = null,
    }: {
      videoId: UUID;
      channelId: UUID;
      platform: Platform;
      platformAccountId?: UUID | null;
      priority?: VideoPriority | null;
    }) => queueApi.addToQueue(workspaceId!, videoId, channelId, platform, platformAccountId, priority),
    onSuccess: invalidate,
  });
}

export function useAddToQueueBulk() {
  const workspaceId = useWorkspaceId();
  const invalidate = useInvalidateQueue();
  return useMutation({
    mutationFn: ({
      videoIds,
      channelId,
      platform,
      platformAccountId = null,
      priority = null,
    }: {
      videoIds: UUID[];
      channelId: UUID;
      platform: Platform;
      platformAccountId?: UUID | null;
      priority?: VideoPriority | null;
    }) => queueApi.addToQueueBulk(workspaceId!, videoIds, channelId, platform, platformAccountId, priority),
    onSuccess: invalidate,
  });
}

export function useCancelPublication() {
  const invalidate = useInvalidateQueue();
  return useMutation({
    mutationFn: (id: UUID) => queueApi.cancel(id),
    onSuccess: invalidate,
  });
}

export function useArchivePublication() {
  const invalidate = useInvalidateQueue();
  return useMutation({
    mutationFn: (id: UUID) => queueApi.archive(id),
    onSuccess: invalidate,
  });
}

export function useSetPublicationPriority() {
  const invalidate = useInvalidateQueue();
  return useMutation({
    mutationFn: ({ id, priority }: { id: UUID; priority: VideoPriority }) =>
      queueApi.setPriority(id, priority),
    onSuccess: invalidate,
  });
}

export function useSetPublicationLocked() {
  const invalidate = useInvalidateQueue();
  return useMutation({
    mutationFn: ({ id, locked }: { id: UUID; locked: boolean }) => queueApi.setLocked(id, locked),
    onSuccess: invalidate,
  });
}

export function useBulkSetPublicationsPaused() {
  const invalidate = useInvalidateQueue();
  return useMutation({
    mutationFn: ({ ids, paused }: { ids: UUID[]; paused: boolean }) => queueApi.bulkSetPaused(ids, paused),
    onSuccess: invalidate,
  });
}

export function useReorderQueue() {
  const invalidate = useInvalidateQueue();
  return useMutation({
    mutationFn: (orderedPublicationIds: UUID[]) => queueApi.reorder(orderedPublicationIds),
    onSuccess: invalidate,
  });
}
