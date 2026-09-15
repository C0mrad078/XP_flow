import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { channelsApi } from "@/lib/tauri";
import { useWorkspaceStore } from "@/stores/workspace-store";

export function useChannels() {
  const workspaceId = useWorkspaceStore((state) => state.workspace?.id);
  return useQuery({
    queryKey: ["channels", workspaceId],
    queryFn: () => channelsApi.list(workspaceId!),
    enabled: Boolean(workspaceId),
  });
}

export function useCreateChannel() {
  const queryClient = useQueryClient();
  const workspaceId = useWorkspaceStore((state) => state.workspace?.id);
  return useMutation({
    mutationFn: (name: string) => channelsApi.create(workspaceId!, name),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["channels", workspaceId] }),
  });
}
