import { BellOff, CheckCircle2, AlertTriangle, XCircle, Info } from "lucide-react";

import { EmptyState } from "@/components/feedback/empty-state";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { formatRelativeTime } from "@/lib/formatting/date";
import { cn } from "@/lib/utilities/cn";
import { useNotificationStore } from "@/stores/notification-store";
import { useUIStore } from "@/stores/ui-store";
import type { NotificationType } from "@/types/domain";

const TYPE_ICON: Record<NotificationType, typeof Info> = {
  info: Info,
  success: CheckCircle2,
  warning: AlertTriangle,
  error: XCircle,
};

const TYPE_CLASSES: Record<NotificationType, string> = {
  info: "text-primary",
  success: "text-success",
  warning: "text-warning",
  error: "text-danger",
};

export function NotificationCenter() {
  const open = useUIStore((state) => state.notificationCenterOpen);
  const setOpen = useUIStore((state) => state.setNotificationCenterOpen);
  const notifications = useNotificationStore((state) => state.notifications);
  const markRead = useNotificationStore((state) => state.markRead);

  return (
    <Sheet open={open} onOpenChange={setOpen}>
      <SheetContent side="right" className="w-full max-w-sm p-0">
        <SheetHeader className="border-b border-border px-5 py-4">
          <SheetTitle>Notifications</SheetTitle>
        </SheetHeader>

        {notifications.length === 0 ? (
          <EmptyState
            icon={BellOff}
            title="No notifications"
            description="You're all caught up. New alerts about your channels and publications will show up here."
          />
        ) : (
          <div className="flex flex-col divide-y divide-border overflow-y-auto">
            {notifications.map((notification) => {
              const Icon = TYPE_ICON[notification.notification_type];
              return (
                <button
                  key={notification.id}
                  type="button"
                  onClick={() => !notification.read && markRead(notification.id)}
                  className={cn(
                    "flex items-start gap-3 px-5 py-3.5 text-left transition-colors hover:bg-surface-hover",
                    !notification.read && "bg-primary/[0.04]",
                  )}
                >
                  <Icon
                    className={cn("mt-0.5 size-4 shrink-0", TYPE_CLASSES[notification.notification_type])}
                  />
                  <div className="min-w-0 flex-1">
                    <p className="text-body-small font-medium text-foreground">{notification.title}</p>
                    <p className="text-body-small text-muted-foreground">{notification.message}</p>
                    <p className="text-caption mt-1">{formatRelativeTime(notification.created_at)}</p>
                  </div>
                  {!notification.read && (
                    <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-primary" aria-hidden />
                  )}
                </button>
              );
            })}
          </div>
        )}
      </SheetContent>
    </Sheet>
  );
}
