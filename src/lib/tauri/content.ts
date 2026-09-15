import { invoke } from "./client";
import type {
  BulkUpdateInput,
  ContentListRequest,
  UpdateVideoInput,
  Video,
  VideoDetail,
  VideoLibrarySummary,
  VideoPage,
} from "@/types/media";
import type { UUID } from "@/types/domain";

export const contentApi = {
  list: (workspaceId: UUID, request: ContentListRequest) =>
    invoke<VideoPage>("list_content", { workspaceId, request }),
  summary: (workspaceId: UUID) => invoke<VideoLibrarySummary>("get_content_summary", { workspaceId }),
  getDetail: (id: UUID) => invoke<VideoDetail | null>("get_video_detail", { id }),
  update: (id: UUID, input: UpdateVideoInput) => invoke<Video>("update_video", { id, input }),
  bulkUpdate: (ids: UUID[], input: BulkUpdateInput) => invoke<number>("bulk_update_videos", { ids, input }),
  setArchived: (id: UUID, archived: boolean) => invoke<Video>("set_video_archived", { id, archived }),
  remove: (id: UUID) => invoke<void>("remove_video", { id }),
  revalidate: (id: UUID) => invoke<Video>("revalidate_video", { id }),
  regenerateThumbnail: (id: UUID) => invoke<Video>("regenerate_video_thumbnail", { id }),
  revealInFileManager: (id: UUID) => invoke<void>("reveal_video_in_file_manager", { id }),
};
