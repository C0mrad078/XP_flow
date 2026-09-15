import { create } from "zustand";

import { notificationsApi } from "@/lib/tauri";
import type { AppNotification } from "@/types/domain";

interface NotificationState {
  notifications: AppNotification[];
  unreadCount: number;
  isLoaded: boolean;
  load: () => Promise<void>;
  markRead: (id: string) => Promise<void>;
}

export const useNotificationStore = create<NotificationState>((set, get) => ({
  notifications: [],
  unreadCount: 0,
  isLoaded: false,

  load: async () => {
    const [notifications, unreadCount] = await Promise.all([
      notificationsApi.listRecent(),
      notificationsApi.unreadCount(),
    ]);
    set({ notifications, unreadCount, isLoaded: true });
  },

  markRead: async (id: string) => {
    await notificationsApi.markRead(id);
    set({
      notifications: get().notifications.map((n) => (n.id === id ? { ...n, read: true } : n)),
      unreadCount: Math.max(0, get().unreadCount - 1),
    });
  },
}));
