import { invoke } from "./client";
import type { Platform, Publication, UUID } from "@/types/domain";
import type {
  ApprovalSource,
  HashtagSet,
  MetadataTemplate,
  MetadataValidationIssue,
  ProviderRateState,
  PublicationAttempt,
  PublishingSettings,
  ReadinessIssue,
  RenderedMetadata,
  TemplateKind,
  UpdatePublicationMetadataInput,
  UpdatePublishingSettingsInput,
} from "@/types/publishing";

export const publishingApi = {
  publishNow: (publicationId: UUID) => invoke<void>("publish_now", { publicationId }),
  retry: (publicationId: UUID) => invoke<void>("retry_publication", { publicationId }),
  attempts: (publicationId: UUID) =>
    invoke<PublicationAttempt[]>("get_publication_attempts", { publicationId }),
  readiness: (publicationId: UUID) =>
    invoke<ReadinessIssue[]>("get_publication_readiness", { publicationId }),
  consent: (publicationId: UUID, approvalSource: ApprovalSource = "manual_schedule") =>
    invoke<void>("record_publication_consent", { publicationId, approvalSource }),
  rateState: (platformAccountId: UUID) =>
    invoke<ProviderRateState[]>("get_provider_rate_state", { platformAccountId }),
  previewMetadata: (publicationId: UUID) =>
    invoke<RenderedMetadata>("preview_publication_metadata", { publicationId }),
  renderedMetadata: (publicationId: UUID) =>
    invoke<RenderedMetadata | null>("get_rendered_metadata", { publicationId }),
  validateMetadata: (publicationId: UUID) =>
    invoke<MetadataValidationIssue[]>("validate_publication_metadata", { publicationId }),
  updateMetadata: (publicationId: UUID, request: UpdatePublicationMetadataInput) =>
    invoke<Publication>("update_publication_metadata", { publicationId, request }),
  settings: () => invoke<PublishingSettings>("get_publishing_settings"),
  updateSettings: (input: UpdatePublishingSettingsInput) =>
    invoke<PublishingSettings>("update_publishing_settings", { input }),
  pause: () => invoke<PublishingSettings>("pause_publishing"),
  resume: () => invoke<PublishingSettings>("resume_publishing"),
  templates: {
    list: (workspaceId: UUID) => invoke<MetadataTemplate[]>("list_metadata_templates", { workspaceId }),
    create: (
      workspaceId: UUID,
      channelId: UUID | null,
      platform: Platform | null,
      kind: TemplateKind,
      templateText: string,
    ) =>
      invoke<MetadataTemplate>("create_metadata_template", {
        workspaceId,
        channelId,
        platform,
        kind,
        templateText,
      }),
    update: (id: UUID, channelId: UUID | null, platform: Platform | null, templateText: string) =>
      invoke<MetadataTemplate>("update_metadata_template", { id, channelId, platform, templateText }),
    remove: (id: UUID) => invoke<void>("delete_metadata_template", { id }),
  },
  hashtags: {
    list: (workspaceId: UUID) => invoke<HashtagSet[]>("list_hashtag_sets", { workspaceId }),
    create: (
      workspaceId: UUID,
      channelId: UUID | null,
      platform: Platform | null,
      name: string,
      hashtags: string[],
    ) => invoke<HashtagSet>("create_hashtag_set", { workspaceId, channelId, platform, name, hashtags }),
    update: (id: UUID, channelId: UUID | null, platform: Platform | null, name: string, hashtags: string[]) =>
      invoke<HashtagSet>("update_hashtag_set", { id, channelId, platform, name, hashtags }),
    remove: (id: UUID) => invoke<void>("delete_hashtag_set", { id }),
  },
};
