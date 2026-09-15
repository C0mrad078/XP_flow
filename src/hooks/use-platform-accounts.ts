import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { platformAccountsApi } from "@/lib/tauri";
import type { UUID } from "@/types/domain";
import { useWorkspaceStore } from "@/stores/workspace-store";

export function usePlatformAccounts(channelId: UUID | null) {
  return useQuery({
    queryKey: ["platform-accounts", channelId],
    queryFn: () => platformAccountsApi.listForChannel(channelId!),
    enabled: Boolean(channelId),
  });
}

export function usePlatformAccountsForWorkspace() {
  const workspaceId = useWorkspaceStore((state) => state.workspace?.id);
  return useQuery({
    queryKey: ["platform-accounts", "workspace", workspaceId],
    queryFn: () => platformAccountsApi.listForWorkspace(workspaceId!),
    enabled: Boolean(workspaceId),
  });
}

function useInvalidatePlatformAccounts() {
  const queryClient = useQueryClient();
  return () => {
    queryClient.invalidateQueries({ queryKey: ["platform-accounts"] });
    queryClient.invalidateQueries({ queryKey: ["channel-overview"] });
  };
}

export function useSetDefaultPlatformAccount() {
  const invalidate = useInvalidatePlatformAccounts();
  return useMutation({
    mutationFn: (id: UUID) => platformAccountsApi.setDefault(id),
    onSuccess: invalidate,
  });
}

export function useReassignPlatformAccountChannel() {
  const invalidate = useInvalidatePlatformAccounts();
  return useMutation({
    mutationFn: ({ id, newChannelId }: { id: UUID; newChannelId: UUID }) =>
      platformAccountsApi.reassignChannel(id, newChannelId),
    onSuccess: invalidate,
  });
}
