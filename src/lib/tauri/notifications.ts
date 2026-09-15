import { invoke } from "./client";
import type { AppNotification } from "@/types/domain";

export const notificationsApi = {
  listRecent: (limit = 50) => invoke<AppNotification[]>("list_recent_notifications", { limit }),
  unreadCount: () => invoke<number>("unread_notification_count"),
  markRead: (id: string) => invoke<void>("mark_notification_read", { id }),
};
