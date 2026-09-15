import { useEffect } from "react";
import { Outlet } from "react-router-dom";

import { CommandPalette } from "@/components/navigation/command-palette";
import { NotificationCenter } from "@/components/navigation/notification-center";
import { Sidebar } from "@/components/navigation/sidebar";
import { TopBar } from "@/components/navigation/top-bar";
import { PageTransition } from "@/components/common/page-transition";
import { useNotificationStore } from "@/stores/notification-store";

export function AppShell() {
  const loadNotifications = useNotificationStore((state) => state.load);

  useEffect(() => {
    loadNotifications();
  }, [loadNotifications]);

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-background text-foreground">
      <Sidebar />
      <div className="flex min-w-0 flex-1 flex-col">
        <TopBar />
        <main className="flex-1 overflow-y-auto">
          <PageTransition>
            <Outlet />
          </PageTransition>
        </main>
      </div>
      <CommandPalette />
      <NotificationCenter />
    </div>
  );
}
