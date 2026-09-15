import type { Platform, PublicationStatus } from "@/types/domain";

import { getMockChannel } from "./channels";
import { hoursFromNow } from "./time";

export type QueuePriority = "low" | "normal" | "high";

export interface MockQueueItem {
  id: string;
  videoTitle: string;
  durationSeconds: number;
  channelId: string;
  channelName: string;
  scheduledAt: string;
  priority: QueuePriority;
  platforms: Partial<Record<Platform, PublicationStatus>>;
}

function channelName(id: string): string {
  return getMockChannel(id)?.name ?? "Unknown channel";
}

/** Mock queue/publication world shared by Today, Queue and the Dashboard
 * "next publications" widget, so the three screens tell one coherent
 * story instead of three unrelated random datasets. Isolated under
 * development/ per section 41 — delete this file the moment the real
 * queue/scheduler lands. */
export const mockQueueItems: MockQueueItem[] = [
  {
    id: "q-1",
    videoTitle: "5 habits that changed my mornings",
    durationSeconds: 47,
    channelId: "ch-motivation",
    channelName: channelName("ch-motivation"),
    scheduledAt: hoursFromNow(-3),
    priority: "normal",
    platforms: { youtube: "published", tiktok: "published" },
  },
  {
    id: "q-2",
    videoTitle: "Why the ocean is salty (30s explainer)",
    durationSeconds: 31,
    channelId: "ch-facts",
    channelName: channelName("ch-facts"),
    scheduledAt: hoursFromNow(-2),
    priority: "normal",
    platforms: { youtube: "published", tiktok: "published", kwai: "published" },
  },
  {
    id: "q-3",
    videoTitle: "One-pan garlic butter shrimp",
    durationSeconds: 52,
    channelId: "ch-cooking",
    channelName: channelName("ch-cooking"),
    scheduledAt: hoursFromNow(-1.5),
    priority: "high",
    platforms: { youtube: "failed" },
  },
  {
    id: "q-4",
    videoTitle: "The compounding trap nobody explains",
    durationSeconds: 58,
    channelId: "ch-finance",
    channelName: channelName("ch-finance"),
    scheduledAt: hoursFromNow(-0.6),
    priority: "high",
    platforms: { youtube: "published", tiktok: "processing" },
  },
  {
    id: "q-5",
    videoTitle: "Golden retriever meets pool for the first time",
    durationSeconds: 24,
    channelId: "ch-pets",
    channelName: channelName("ch-pets"),
    scheduledAt: hoursFromNow(-0.2),
    priority: "normal",
    platforms: { tiktok: "uploading", kwai: "queued" },
  },
  {
    id: "q-6",
    videoTitle: "This clutch 1v4 shouldn't have worked",
    durationSeconds: 39,
    channelId: "ch-gaming",
    channelName: channelName("ch-gaming"),
    scheduledAt: hoursFromNow(0.3),
    priority: "high",
    platforms: { youtube: "auth_required", tiktok: "auth_required", kwai: "auth_required" },
  },
  {
    id: "q-7",
    videoTitle: "Evening reset routine for deep sleep",
    durationSeconds: 44,
    channelId: "ch-motivation",
    channelName: channelName("ch-motivation"),
    scheduledAt: hoursFromNow(1.5),
    priority: "normal",
    platforms: { youtube: "scheduled", tiktok: "scheduled" },
  },
  {
    id: "q-8",
    videoTitle: "Why cats knead blankets",
    durationSeconds: 28,
    channelId: "ch-facts",
    channelName: channelName("ch-facts"),
    scheduledAt: hoursFromNow(2.5),
    priority: "low",
    platforms: { youtube: "scheduled", tiktok: "scheduled", kwai: "scheduled" },
  },
  {
    id: "q-9",
    videoTitle: "15-minute high protein lunch box",
    durationSeconds: 61,
    channelId: "ch-cooking",
    channelName: channelName("ch-cooking"),
    scheduledAt: hoursFromNow(4),
    priority: "normal",
    platforms: { youtube: "queued" },
  },
  {
    id: "q-10",
    videoTitle: "Rate limited on TikTok — auto retry in queue",
    durationSeconds: 36,
    channelId: "ch-finance",
    channelName: channelName("ch-finance"),
    scheduledAt: hoursFromNow(5),
    priority: "normal",
    platforms: { tiktok: "rate_limited" },
  },
  {
    id: "q-11",
    videoTitle: "Puppy's first snow day",
    durationSeconds: 19,
    channelId: "ch-pets",
    channelName: channelName("ch-pets"),
    scheduledAt: hoursFromNow(6.5),
    priority: "low",
    platforms: { tiktok: "queued", kwai: "queued" },
  },
  {
    id: "q-12",
    videoTitle: "Ranked grind highlight reel #42",
    durationSeconds: 33,
    channelId: "ch-gaming",
    channelName: channelName("ch-gaming"),
    scheduledAt: hoursFromNow(8),
    priority: "normal",
    platforms: { youtube: "paused", tiktok: "paused", kwai: "paused" },
  },
  {
    id: "q-13",
    videoTitle: "The 2-minute rule for procrastination",
    durationSeconds: 41,
    channelId: "ch-motivation",
    channelName: channelName("ch-motivation"),
    scheduledAt: hoursFromNow(26),
    priority: "normal",
    platforms: { youtube: "scheduled", tiktok: "scheduled" },
  },
  {
    id: "q-14",
    videoTitle: "How glass is actually made",
    durationSeconds: 29,
    channelId: "ch-facts",
    channelName: channelName("ch-facts"),
    scheduledAt: hoursFromNow(30),
    priority: "low",
    platforms: { youtube: "ready", tiktok: "ready", kwai: "ready" },
  },
  {
    id: "q-15",
    videoTitle: "Crispy smashed potatoes, 3 ways",
    durationSeconds: 55,
    channelId: "ch-cooking",
    channelName: channelName("ch-cooking"),
    scheduledAt: hoursFromNow(48),
    priority: "normal",
    platforms: { youtube: "imported" },
  },
];

export function getTodayQueueItems(now: Date = new Date()): MockQueueItem[] {
  return mockQueueItems.filter((item) => sameDay(new Date(item.scheduledAt), now));
}

function sameDay(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}
