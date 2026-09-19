import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { analyticsApi } from "@/lib/tauri";
import type { Platform, UUID } from "@/types/domain";
import { useWorkspaceStore } from "@/stores/workspace-store";
export function useAnalyticsCapabilities(platform: Platform) {
  return useQuery({
    queryKey: ["analytics-capabilities", platform],
    queryFn: () => analyticsApi.capabilities(platform),
    staleTime: 300_000,
  });
}
export function usePublicationAnalytics(id: UUID | null, days = 30) {
  return useQuery({
    queryKey: ["publication-analytics", id, days],
    queryFn: () => analyticsApi.publication(id!, days),
    enabled: Boolean(id),
    staleTime: 60_000,
  });
}
export function useSyncPublicationAnalytics() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: UUID) => analyticsApi.syncPublication(id),
    onSuccess: (_snapshot, id) => {
      void queryClient.invalidateQueries({ queryKey: ["publication-analytics", id] });
    },
  });
}

export function useSyncWorkspaceAnalytics() {
  const workspaceId = useWorkspaceStore((state) => state.workspace?.id);
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: () => analyticsApi.syncWorkspace(workspaceId!),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["publication-analytics"] });
    },
  });
}
