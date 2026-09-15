import type { PublicationStatus } from "@/types/domain";

import { getMockChannel } from "./channels";
import { hoursFromNow } from "./time";

export interface MockContentItem {
  id: string;
  title: string;
  durationSeconds: number;
  channelId: string;
  source: "local_file" | "cut_pro_export" | "manual_upload";
  status: PublicationStatus;
  importedAt: string;
  fileSizeBytes: number;
}

function channelName(id: string): string {
  return getMockChannel(id)?.name ?? "Unassigned";
}

/** Mock Content Library rows (section 28) — isolated from `queue.ts`
 * because a video's presence in the library is independent of any
 * publication schedule. Delete once `VideoRepository` is wired to a real
 * screen. */
export const mockContentItems: MockContentItem[] = [
  {
    id: "v-1",
    title: "5 habits that changed my mornings",
    durationSeconds: 47,
    channelId: "ch-motivation",
    source: "cut_pro_export",
    status: "published",
    importedAt: hoursFromNow(-30),
    fileSizeBytes: 41_200_000,
  },
  {
    id: "v-2",
    title: "Why the ocean is salty (30s explainer)",
    durationSeconds: 31,
    channelId: "ch-facts",
    source: "cut_pro_export",
    status: "published",
    importedAt: hoursFromNow(-28),
    fileSizeBytes: 26_800_000,
  },
  {
    id: "v-3",
    title: "One-pan garlic butter shrimp",
    durationSeconds: 52,
    channelId: "ch-cooking",
    source: "manual_upload",
    status: "failed",
    importedAt: hoursFromNow(-20),
    fileSizeBytes: 58_400_000,
  },
  {
    id: "v-4",
    title: "The compounding trap nobody explains",
    durationSeconds: 58,
    channelId: "ch-finance",
    source: "cut_pro_export",
    status: "published",
    importedAt: hoursFromNow(-18),
    fileSizeBytes: 61_900_000,
  },
  {
    id: "v-5",
    title: "Golden retriever meets pool for the first time",
    durationSeconds: 24,
    channelId: "ch-pets",
    source: "local_file",
    status: "uploading",
    importedAt: hoursFromNow(-4),
    fileSizeBytes: 19_700_000,
  },
  {
    id: "v-6",
    title: "This clutch 1v4 shouldn't have worked",
    durationSeconds: 39,
    channelId: "ch-gaming",
    source: "cut_pro_export",
    status: "auth_required",
    importedAt: hoursFromNow(-3),
    fileSizeBytes: 33_100_000,
  },
  {
    id: "v-7",
    title: "Evening reset routine for deep sleep",
    durationSeconds: 44,
    channelId: "ch-motivation",
    source: "cut_pro_export",
    status: "scheduled",
    importedAt: hoursFromNow(-2),
    fileSizeBytes: 37_600_000,
  },
  {
    id: "v-8",
    title: "Why cats knead blankets",
    durationSeconds: 28,
    channelId: "ch-facts",
    source: "manual_upload",
    status: "scheduled",
    importedAt: hoursFromNow(-1),
    fileSizeBytes: 22_300_000,
  },
  {
    id: "v-9",
    title: "Crispy smashed potatoes, 3 ways",
    durationSeconds: 55,
    channelId: "ch-cooking",
    source: "cut_pro_export",
    status: "imported",
    importedAt: hoursFromNow(-0.5),
    fileSizeBytes: 49_500_000,
  },
  {
    id: "v-10",
    title: "Ranked grind highlight reel #42",
    durationSeconds: 33,
    channelId: "ch-gaming",
    source: "local_file",
    status: "ready",
    importedAt: hoursFromNow(-0.2),
    fileSizeBytes: 28_900_000,
  },
];

export { channelName as mockContentChannelName };
