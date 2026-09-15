import { Bell } from "lucide-react";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { useNotificationStore } from "@/stores/notification-store";

export function NotificationsSection() {
  const unreadCount = useNotificationStore((state) => state.unreadCount);

  return (
    <Card>
      <CardHeader>
        <CardTitle>Notification Center</CardTitle>
        <CardDescription>
          In-app notifications are on by default and stored locally. Native OS notifications aren't
          implemented yet (section 40).
        </CardDescription>
      </CardHeader>
      <CardContent className="flex items-center gap-3">
        <div className="flex size-9 items-center justify-center rounded-lg bg-primary/10 text-primary">
          <Bell className="size-4" />
        </div>
        <div>
          <p className="text-body-small font-medium text-foreground">{unreadCount} unread</p>
          <p className="text-caption normal-case tracking-normal">
            Open via the bell icon in the top bar, or ⌘K.
          </p>
        </div>
      </CardContent>
    </Card>
  );
}
