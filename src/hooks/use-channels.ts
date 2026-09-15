import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { channelsApi, type ChannelStatus } from "@/lib/tauri";
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

export function useSetChannelStatus() {
  const queryClient = useQueryClient();
  const workspaceId = useWorkspaceStore((state) => state.workspace?.id);
  return useMutation({
    mutationFn: ({ id, status }: { id: string; status: ChannelStatus }) => channelsApi.setStatus(id, status),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["channels", workspaceId] });
      queryClient.invalidateQueries({ queryKey: ["channel-overview", workspaceId] });
    },
  });
}

/** Section 96 — one query backing the whole Channels screen instead of
 * 3 IPC round trips per rendered card. */
export function useChannelOverview() {
  const workspaceId = useWorkspaceStore((state) => state.workspace?.id);
  return useQuery({
    queryKey: ["channel-overview", workspaceId],
    queryFn: () => channelsApi.operationalOverview(workspaceId!),
    enabled: Boolean(workspaceId),
  });
}
