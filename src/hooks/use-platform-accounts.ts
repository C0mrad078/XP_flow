import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";

import { platformAccountsApi } from "@/lib/tauri";
import type { Platform, UUID } from "@/types/domain";

export function usePlatformAccounts(channelId: UUID | null) {
  return useQuery({
    queryKey: ["platform-accounts", channelId],
    queryFn: () => platformAccountsApi.listForChannel(channelId!),
    enabled: Boolean(channelId),
  });
}

export function useCreatePlatformAccount(channelId: UUID) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (platform: Platform) => platformAccountsApi.create(channelId, platform),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["platform-accounts", channelId] }),
  });
}

export function useSetDefaultPlatformAccount(channelId: UUID) {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (id: UUID) => platformAccountsApi.setDefault(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["platform-accounts", channelId] }),
  });
}
