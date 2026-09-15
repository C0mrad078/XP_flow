import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { sourcesApi } from "@/lib/tauri";
import { useWorkspaceStore } from "@/stores/workspace-store";
import type { CreateSourceInput, UpdateSourceInput } from "@/types/media";

function useWorkspaceId(): string | undefined {
  return useWorkspaceStore((state) => state.workspace?.id);
}

export function useSources() {
  const workspaceId = useWorkspaceId();
  return useQuery({
    queryKey: ["sources", workspaceId],
    queryFn: () => sourcesApi.list(workspaceId!),
    enabled: Boolean(workspaceId),
    refetchInterval: 15_000,
  });
}

function useInvalidateSources() {
  const queryClient = useQueryClient();
  const workspaceId = useWorkspaceId();
  return () => queryClient.invalidateQueries({ queryKey: ["sources", workspaceId] });
}

export function useCreateSource() {
  const invalidate = useInvalidateSources();
  const workspaceId = useWorkspaceId();
  return useMutation({
    mutationFn: (input: CreateSourceInput) => sourcesApi.create(workspaceId!, input),
    onSuccess: invalidate,
  });
}

export function useUpdateSource() {
  const invalidate = useInvalidateSources();
  return useMutation({
    mutationFn: ({ id, input }: { id: string; input: UpdateSourceInput }) => sourcesApi.update(id, input),
    onSuccess: invalidate,
  });
}

export function useDeleteSource() {
  const invalidate = useInvalidateSources();
  return useMutation({
    mutationFn: (id: string) => sourcesApi.remove(id),
    onSuccess: invalidate,
  });
}

export function useScanSourceNow() {
  const invalidate = useInvalidateSources();
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => sourcesApi.scanNow(id),
    onSuccess: () => {
      invalidate();
      queryClient.invalidateQueries({ queryKey: ["content"] });
      queryClient.invalidateQueries({ queryKey: ["content-summary"] });
    },
  });
}
