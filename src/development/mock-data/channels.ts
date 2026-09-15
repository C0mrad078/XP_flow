import type { Platform } from "@/types/domain";

export type ChannelHealth = "healthy" | "attention" | "critical";

export interface MockChannelConnection {
  connected: boolean;
  handle?: string;
}

export interface MockChannel {
  id: string;
  name: string;
  niche: string;
  connections: Record<Platform, MockChannelConnection>;
  queuedVideos: number;
  contentStockDays: number;
  health: ChannelHealth;
}

/** Mock-only channel roster for the Channels screen (section 29). Real
 * channel persistence already exists on the backend
 * (`infrastructure::repositories::SqliteChannelRepository`) — this file is
 * deliberately isolated so it can be deleted the moment the Channels
 * screen is wired to real data. */
export const mockChannels: MockChannel[] = [
  {
    id: "ch-motivation",
    name: "Momentum Daily",
    niche: "Motivation",
    connections: {
      youtube: { connected: true, handle: "@momentumdaily" },
      tiktok: { connected: true, handle: "@momentumdaily" },
      kwai: { connected: false },
    },
    queuedVideos: 18,
    contentStockDays: 12,
    health: "healthy",
  },
  {
    id: "ch-facts",
    name: "Quick Facts Lab",
    niche: "Educational",
    connections: {
      youtube: { connected: true, handle: "@quickfactslab" },
      tiktok: { connected: true, handle: "@quickfactslab" },
      kwai: { connected: true, handle: "@quickfactslab" },
    },
    queuedVideos: 7,
    contentStockDays: 4,
    health: "attention",
  },
  {
    id: "ch-cooking",
    name: "60 Second Kitchen",
    niche: "Cooking",
    connections: {
      youtube: { connected: true, handle: "@60secondkitchen" },
      tiktok: { connected: false },
      kwai: { connected: false },
    },
    queuedVideos: 3,
    contentStockDays: 1,
    health: "critical",
  },
  {
    id: "ch-finance",
    name: "Wealth Signals",
    niche: "Finance",
    connections: {
      youtube: { connected: true, handle: "@wealthsignals" },
      tiktok: { connected: true, handle: "@wealthsignals" },
      kwai: { connected: false },
    },
    queuedVideos: 22,
    contentStockDays: 19,
    health: "healthy",
  },
  {
    id: "ch-pets",
    name: "Paws & Clips",
    niche: "Pets",
    connections: {
      youtube: { connected: false },
      tiktok: { connected: true, handle: "@pawsandclips" },
      kwai: { connected: true, handle: "@pawsandclips" },
    },
    queuedVideos: 11,
    contentStockDays: 8,
    health: "healthy",
  },
  {
    id: "ch-gaming",
    name: "Clutch Reel",
    niche: "Gaming",
    connections: {
      youtube: { connected: true, handle: "@clutchreel" },
      tiktok: { connected: true, handle: "@clutchreel" },
      kwai: { connected: true, handle: "@clutchreel" },
    },
    queuedVideos: 0,
    contentStockDays: 0,
    health: "critical",
  },
];

export function getMockChannel(id: string): MockChannel | undefined {
  return mockChannels.find((channel) => channel.id === id);
}
