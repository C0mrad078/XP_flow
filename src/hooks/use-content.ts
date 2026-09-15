import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { contentApi } from "@/lib/tauri";
import { useContentStore } from "@/stores/content-store";
import { useWorkspaceStore } from "@/stores/workspace-store";
import type { BulkUpdateInput, ContentListRequest, UpdateVideoInput } from "@/types/media";

const PAGE_SIZE = 60;

function useWorkspaceId(): string | undefined {
  return useWorkspaceStore((state) => state.workspace?.id);
}

function toRequest(filters: ReturnType<typeof useContentStore.getState>["filters"]): ContentListRequest {
  return {
    search: filters.search || undefined,
    channel_id: filters.channelId,
    unassigned_only: filters.unassignedOnly || undefined,
    source_id: filters.sourceId,
    validation_status: filters.validationStatus,
    availability_status: filters.availabilityStatus,
    orientation: filters.orientation,
    priority: filters.priority,
    possible_duplicates_only: filters.possibleDuplicatesOnly || undefined,
    include_archived: filters.includeArchived || undefined,
    sort: filters.sort,
    page: filters.page,
    page_size: PAGE_SIZE,
  };
}

export function useContentLibrary() {
  const workspaceId = useWorkspaceId();
  const filters = useContentStore((state) => state.filters);

  return useQuery({
    queryKey: ["content", workspaceId, filters],
    queryFn: () => contentApi.list(workspaceId!, toRequest(filters)),
    enabled: Boolean(workspaceId),
    placeholderData: (previous) => previous,
  });
}

export function useContentSummary() {
  const workspaceId = useWorkspaceId();
  return useQuery({
    queryKey: ["content-summary", workspaceId],
    queryFn: () => contentApi.summary(workspaceId!),
    enabled: Boolean(workspaceId),
    refetchInterval: 10_000,
  });
}

export function useVideoDetail(id: string | null) {
  return useQuery({
    queryKey: ["video-detail", id],
    queryFn: () => contentApi.getDetail(id!),
    enabled: Boolean(id),
  });
}

function useInvalidateContent() {
  const queryClient = useQueryClient();
  return () => {
    queryClient.invalidateQueries({ queryKey: ["content"] });
    queryClient.invalidateQueries({ queryKey: ["content-summary"] });
  };
}

export function useUpdateVideo() {
  const invalidate = useInvalidateContent();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateVideoInput }) => contentApi.update(id, input),
    onSuccess: (video) => {
      queryClient.invalidateQueries({ queryKey: ["video-detail", video.id] });
      invalidate();
    },
  });
}

export function useBulkUpdateVideos() {
  const invalidate = useInvalidateContent();
  return useMutation({
    mutationFn: ({ ids, input }: { ids: string[]; input: BulkUpdateInput }) => contentApi.bulkUpdate(ids, input),
    onSuccess: invalidate,
  });
}

export function useSetVideoArchived() {
  const invalidate = useInvalidateContent();
  return useMutation({
    mutationFn: ({ id, archived }: { id: string; archived: boolean }) => contentApi.setArchived(id, archived),
    onSuccess: invalidate,
  });
}

export function useRemoveVideo() {
  const invalidate = useInvalidateContent();
  return useMutation({
    mutationFn: (id: string) => contentApi.remove(id),
    onSuccess: invalidate,
  });
}

export function useRevalidateVideo() {
  const invalidate = useInvalidateContent();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => contentApi.revalidate(id),
    onSuccess: (video) => {
      queryClient.invalidateQueries({ queryKey: ["video-detail", video.id] });
      invalidate();
    },
  });
}

export function useRegenerateThumbnail() {
  const invalidate = useInvalidateContent();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => contentApi.regenerateThumbnail(id),
    onSuccess: (video) => {
      queryClient.invalidateQueries({ queryKey: ["video-detail", video.id] });
      invalidate();
    },
  });
}
