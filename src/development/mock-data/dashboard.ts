import type { Platform } from "@/types/domain";

export interface DashboardMetrics {
  totalViews: number;
  totalViewsTrend: number;
  publishedToday: number;
  publishedTodayTarget: number;
  queuedPosts: number;
  activeChannels: number;
  totalChannels: number;
}

export const mockDashboardMetrics: DashboardMetrics = {
  totalViews: 4_218_940,
  totalViewsTrend: 12.4,
  publishedToday: 4,
  publishedTodayTarget: 9,
  queuedPosts: 41,
  activeChannels: 5,
  totalChannels: 6,
};

/** 14 days of a single rolled-up view-count series, used only to give the
 * Dashboard's chart placeholder (section 24) a believable shape. */
export const mockViewsSeries: number[] = [
  18200, 21400, 19800, 24100, 26700, 25300, 29800, 31200, 28600, 34100, 36900, 33400, 39200, 41800,
];

export interface PlatformOverviewRow {
  platform: Platform;
  publications: number;
  views: number;
  share: number;
}

export const mockPlatformOverview: PlatformOverviewRow[] = [
  { platform: "youtube", publications: 214, views: 2_318_400, share: 55 },
  { platform: "tiktok", publications: 268, views: 1_512_900, share: 36 },
  { platform: "kwai", publications: 96, views: 387_640, share: 9 },
];
