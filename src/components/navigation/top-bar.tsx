import { Bell, LayoutDashboard, Search } from "lucide-react";
import { useLocation } from "react-router-dom";

import { ALL_NAV_ITEMS } from "@/app/routes/nav-items";
import { IconButton } from "@/components/ui/icon-button";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utilities/cn";
import { useNotificationStore } from "@/stores/notification-store";
import { useUIStore } from "@/stores/ui-store";
import { useWorkspaceStore } from "@/stores/workspace-store";

function usePageTitle(): { title: string; icon: typeof LayoutDashboard } {
  const { pathname } = useLocation();
  if (pathname === "/") return { title: "Dashboard", icon: LayoutDashboard };
  const match = ALL_NAV_ITEMS.find((item) => pathname.startsWith(item.path));
  return { title: match?.label ?? "XP FLOW", icon: match?.icon ?? LayoutDashboard };
}

export function TopBar() {
  const { title, icon: Icon } = usePageTitle();
  const setCommandPaletteOpen = useUIStore((state) => state.setCommandPaletteOpen);
  const setNotificationCenterOpen = useUIStore((state) => state.setNotificationCenterOpen);
  const unreadCount = useNotificationStore((state) => state.unreadCount);
  const workspace = useWorkspaceStore((state) => state.workspace);

  return (
    <header
      data-tauri-drag-region
      className="flex h-14 shrink-0 items-center justify-between gap-4 border-b border-border bg-background px-5"
    >
      <div className="flex min-w-0 items-center gap-2.5">
        <Icon className="size-4 text-muted-foreground" />
        <h1 className="text-page-title truncate text-foreground">{title}</h1>
      </div>

      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={() => setCommandPaletteOpen(true)}
          className={cn(
            "flex h-8 w-56 items-center gap-2 rounded-md border border-border bg-surface px-2.5 text-left",
            "text-body-small text-muted transition-colors hover:border-border-strong hover:bg-surface-hover",
          )}
        >
          <Search className="size-3.5 shrink-0" />
          <span className="flex-1 truncate">Search or jump to…</span>
          <kbd className="rounded border border-border bg-surface-elevated px-1.5 py-0.5 font-mono-data text-[0.6875rem] text-muted">
            ⌘K
          </kbd>
        </button>

        <IconButton
          label={unreadCount > 0 ? `Notifications (${unreadCount} unread)` : "Notifications"}
          onClick={() => setNotificationCenterOpen(true)}
          className="relative"
        >
          <Bell className="size-4" />
          {unreadCount > 0 && (
            <Badge
              variant="primary"
              className="absolute -right-0.5 -top-0.5 h-4 min-w-4 justify-center border-0 px-1 py-0 text-[0.625rem]"
            >
              {unreadCount > 9 ? "9+" : unreadCount}
            </Badge>
          )}
        </IconButton>

        <Avatar className="size-8">
          <AvatarFallback>{(workspace?.name ?? "XP").slice(0, 2).toUpperCase()}</AvatarFallback>
        </Avatar>
      </div>
    </header>
  );
}
