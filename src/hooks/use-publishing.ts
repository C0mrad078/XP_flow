import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { publishingApi } from "@/lib/tauri";
import { useWorkspaceStore } from "@/stores/workspace-store";
import type { UUID } from "@/types/domain";
import type {
  ApprovalSource,
  UpdatePublicationMetadataInput,
  UpdatePublishingSettingsInput,
} from "@/types/publishing";

const publicationKey = (id: UUID | null) => ["publication", id] as const;
function invalidatePublication(queryClient: ReturnType<typeof useQueryClient>, id: UUID) {
  queryClient.invalidateQueries({ queryKey: publicationKey(id) });
  queryClient.invalidateQueries({ queryKey: ["queue"] });
  queryClient.invalidateQueries({ queryKey: ["calendar"] });
  queryClient.invalidateQueries({ queryKey: ["today"] });
}

export function usePublicationReadiness(id: UUID | null) {
  return useQuery({
    queryKey: ["publication-readiness", id],
    queryFn: () => publishingApi.readiness(id!),
    enabled: Boolean(id),
    staleTime: 10_000,
  });
}
export function usePublicationAttempts(id: UUID | null) {
  return useQuery({
    queryKey: ["publication-attempts", id],
    queryFn: () => publishingApi.attempts(id!),
    enabled: Boolean(id),
    refetchInterval: (query) => (query.state.data?.some((a) => a.status === "running") ? 2_000 : false),
  });
}
export function usePublicationMetadata(id: UUID | null) {
  return useQuery({
    queryKey: ["publication-metadata", id],
    queryFn: () => publishingApi.previewMetadata(id!),
    enabled: Boolean(id),
    staleTime: 5_000,
  });
}
export function useProviderRateState(platformAccountId: UUID | null) {
  return useQuery({
    queryKey: ["publishing-rate-state", platformAccountId],
    queryFn: () => publishingApi.rateState(platformAccountId!),
    enabled: Boolean(platformAccountId),
    staleTime: 15_000,
  });
}
export function useRenderedMetadata(id: UUID | null) {
  return useQuery({
    queryKey: ["rendered-metadata", id],
    queryFn: () => publishingApi.renderedMetadata(id!),
    enabled: Boolean(id),
  });
}
export function usePublishNow() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: UUID) => publishingApi.publishNow(id),
    onSuccess: (_, id) => invalidatePublication(qc, id),
  });
}
export function useRetryPublication() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: UUID) => publishingApi.retry(id),
    onSuccess: (_, id) => invalidatePublication(qc, id),
  });
}
export function useReconcilePublication() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: UUID) => publishingApi.reconcile(id),
    onSuccess: (_, id) => invalidatePublication(qc, id),
  });
}
export function useRepostPublication() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: UUID) => publishingApi.repost(id),
    onSuccess: (_, id) => invalidatePublication(qc, id),
  });
}
export function useRecordPublicationConsent() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, source }: { id: UUID; source?: ApprovalSource }) => publishingApi.consent(id, source),
    onSuccess: (_, { id }) => {
      invalidatePublication(qc, id);
      qc.invalidateQueries({ queryKey: ["publication-readiness", id] });
    },
  });
}
export function useUpdatePublicationMetadata() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, request }: { id: UUID; request: UpdatePublicationMetadataInput }) =>
      publishingApi.updateMetadata(id, request),
    onSuccess: (_, { id }) => {
      invalidatePublication(qc, id);
      qc.invalidateQueries({ queryKey: ["publication-metadata", id] });
      qc.invalidateQueries({ queryKey: ["rendered-metadata", id] });
    },
  });
}
export function usePublishingSettings() {
  return useQuery({ queryKey: ["publishing-settings"], queryFn: publishingApi.settings });
}
export function useUpdatePublishingSettings() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: UpdatePublishingSettingsInput) => publishingApi.updateSettings(input),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["publishing-settings"] }),
  });
}
export function usePausePublishing() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: publishingApi.pause,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["publishing-settings"] }),
  });
}
export function useResumePublishing() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: publishingApi.resume,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["publishing-settings"] }),
  });
}
export function useMetadataTemplates() {
  const workspaceId = useWorkspaceStore((s) => s.workspace?.id);
  return useQuery({
    queryKey: ["metadata-templates", workspaceId],
    queryFn: () => publishingApi.templates.list(workspaceId!),
    enabled: Boolean(workspaceId),
  });
}
export function useHashtagSets() {
  const workspaceId = useWorkspaceStore((s) => s.workspace?.id);
  return useQuery({
    queryKey: ["hashtag-sets", workspaceId],
    queryFn: () => publishingApi.hashtags.list(workspaceId!),
    enabled: Boolean(workspaceId),
  });
}
export function useCreateMetadataTemplate() {
  const qc = useQueryClient();
  const workspaceId = useWorkspaceStore((s) => s.workspace?.id);
  return useMutation({
    mutationFn: ({ kind, text }: { kind: "title" | "description"; text: string }) =>
      publishingApi.templates.create(workspaceId!, null, null, kind, text),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["metadata-templates", workspaceId] }),
  });
}
export function useDeleteMetadataTemplate() {
  const qc = useQueryClient();
  const workspaceId = useWorkspaceStore((s) => s.workspace?.id);
  return useMutation({
    mutationFn: publishingApi.templates.remove,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["metadata-templates", workspaceId] }),
  });
}
export function useCreateHashtagSet() {
  const qc = useQueryClient();
  const workspaceId = useWorkspaceStore((s) => s.workspace?.id);
  return useMutation({
    mutationFn: ({ name, hashtags }: { name: string; hashtags: string[] }) =>
      publishingApi.hashtags.create(workspaceId!, null, null, name, hashtags),
    onSuccess: () => qc.invalidateQueries({ queryKey: ["hashtag-sets", workspaceId] }),
  });
}
export function useDeleteHashtagSet() {
  const qc = useQueryClient();
  const workspaceId = useWorkspaceStore((s) => s.workspace?.id);
  return useMutation({
    mutationFn: publishingApi.hashtags.remove,
    onSuccess: () => qc.invalidateQueries({ queryKey: ["hashtag-sets", workspaceId] }),
  });
}
