import { lazy, Suspense, useEffect } from "react";
import { Navigate, Route, Routes } from "react-router-dom";

import { AppShell } from "@/app/layouts/app-shell";
import { AppProviders } from "@/app/providers/app-providers";
import { ConfirmDialogHost } from "@/components/feedback/confirm-dialog";
import { Toaster } from "@/components/feedback/toaster";
import { LoadingState } from "@/components/feedback/loading-state";
import { OnboardingScreen } from "@/features/onboarding/onboarding-screen";
import { useSettingsStore } from "@/stores/settings-store";
import { useWorkspaceStore } from "@/stores/workspace-store";

// Route-level code splitting (section 51: "use lazy loading where
// useful") — the shell, onboarding and dashboard load eagerly since
// they're needed immediately; every other screen loads on first visit.
const DashboardPage = lazy(() =>
  import("@/features/dashboard/dashboard-page").then((m) => ({ default: m.DashboardPage })),
);
const TodayPage = lazy(() => import("@/features/today/today-page").then((m) => ({ default: m.TodayPage })));
const QueuePage = lazy(() => import("@/features/queue/queue-page").then((m) => ({ default: m.QueuePage })));
const ContentPage = lazy(() =>
  import("@/features/content/content-page").then((m) => ({ default: m.ContentPage })),
);
const CalendarPage = lazy(() =>
  import("@/features/calendar/calendar-page").then((m) => ({ default: m.CalendarPage })),
);
const ChannelsPage = lazy(() =>
  import("@/features/channels/channels-page").then((m) => ({ default: m.ChannelsPage })),
);
const CommentsPage = lazy(() =>
  import("@/features/comments/comments-page").then((m) => ({ default: m.CommentsPage })),
);
const AnalyticsPage = lazy(() =>
  import("@/features/analytics/analytics-page").then((m) => ({ default: m.AnalyticsPage })),
);
const ActivityPage = lazy(() =>
  import("@/features/activity/activity-page").then((m) => ({ default: m.ActivityPage })),
);
const AutomationPage = lazy(() =>
  import("@/features/automation/automation-page").then((m) => ({ default: m.AutomationPage })),
);
const SettingsPage = lazy(() =>
  import("@/features/settings/settings-page").then((m) => ({ default: m.SettingsPage })),
);

function AppContent() {
  const loadWorkspace = useWorkspaceStore((state) => state.load);
  const workspace = useWorkspaceStore((state) => state.workspace);
  const workspaceLoaded = useWorkspaceStore((state) => state.isLoaded);
  const loadSettings = useSettingsStore((state) => state.load);
  const settingsLoaded = useSettingsStore((state) => state.isLoaded);

  useEffect(() => {
    loadWorkspace();
    loadSettings();
  }, [loadWorkspace, loadSettings]);

  if (!workspaceLoaded || !settingsLoaded) {
    return (
      <div className="flex h-screen w-screen items-center justify-center bg-background">
        <LoadingState label="Starting XP FLOW…" />
      </div>
    );
  }

  if (!workspace) {
    return <OnboardingScreen />;
  }

  return (
    <Suspense fallback={<LoadingState label="Loading…" className="py-24" />}>
      <Routes>
        <Route element={<AppShell />}>
          <Route index element={<DashboardPage />} />
          <Route path="today" element={<TodayPage />} />
          <Route path="queue" element={<QueuePage />} />
          <Route path="content" element={<ContentPage />} />
          <Route path="calendar" element={<CalendarPage />} />
          <Route path="channels" element={<ChannelsPage />} />
          <Route path="comments" element={<CommentsPage />} />
          <Route path="analytics" element={<AnalyticsPage />} />
          <Route path="activity" element={<ActivityPage />} />
          <Route path="automation" element={<AutomationPage />} />
          <Route path="settings" element={<SettingsPage />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Route>
      </Routes>
    </Suspense>
  );
}

export function App() {
  return (
    <AppProviders>
      <AppContent />
      <Toaster />
      <ConfirmDialogHost />
    </AppProviders>
  );
}
