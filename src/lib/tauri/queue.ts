import { invoke } from "./client";
import type { Platform, Publication, UUID } from "@/types/domain";
import type { VideoPriority } from "@/types/media";
import type { AddToQueueOutcome, PublicationListRequest, PublicationPage } from "@/types/scheduling";

export const queueApi = {
  list: (workspaceId: UUID, request: PublicationListRequest) =>
    invoke<PublicationPage>("list_publications", { workspaceId, request }),
  get: (id: UUID) => invoke<Publication | null>("get_publication", { id }),
  addToQueue: (
    workspaceId: UUID,
    videoId: UUID,
    channelId: UUID,
    platform: Platform,
    platformAccountId: UUID | null,
    priority: VideoPriority | null,
  ) =>
    invoke<Publication>("add_to_queue", {
      workspaceId,
      videoId,
      channelId,
      platform,
      platformAccountId,
      priority,
    }),
  addToQueueBulk: (
    workspaceId: UUID,
    videoIds: UUID[],
    channelId: UUID,
    platform: Platform,
    platformAccountId: UUID | null,
    priority: VideoPriority | null,
  ) =>
    invoke<AddToQueueOutcome[]>("add_to_queue_bulk", {
      workspaceId,
      videoIds,
      channelId,
      platform,
      platformAccountId,
      priority,
    }),
  cancel: (id: UUID) => invoke<Publication>("cancel_publication", { id }),
  archive: (id: UUID) => invoke<Publication>("archive_publication", { id }),
  setPriority: (id: UUID, priority: VideoPriority) =>
    invoke<Publication>("set_publication_priority", { id, priority }),
  setLocked: (id: UUID, locked: boolean) => invoke<Publication>("set_publication_locked", { id, locked }),
  reorder: (orderedPublicationIds: UUID[]) => invoke<void>("reorder_queue", { orderedPublicationIds }),
};
