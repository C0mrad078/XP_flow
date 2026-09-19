import { invoke } from "./client";
import type { Platform, UUID } from "@/types/domain";
export interface AnalyticsCapabilities {
  platform: Platform;
  publication_views: boolean;
  publication_likes: boolean;
  publication_comments: boolean;
  publication_shares: boolean;
  channel_followers: boolean;
}
export interface PublicationMetricSnapshot {
  id: UUID;
  publication_id: UUID;
  provider: Platform;
  captured_at: string;
  views: number | null;
  likes: number | null;
  comments: number | null;
  shares: number | null;
  availability: "supported" | "unsupported" | "unavailable" | "failed";
  error_code: string | null;
}
export const analyticsApi = {
  capabilities: (platform: Platform) =>
    invoke<AnalyticsCapabilities>("get_analytics_capabilities", { platform }),
  publication: (publicationId: UUID, days = 30) =>
    invoke<PublicationMetricSnapshot[]>("list_publication_analytics", { publicationId, days }),
  syncPublication: (publicationId: UUID) =>
    invoke<PublicationMetricSnapshot>("sync_publication_analytics", { publicationId }),
};
