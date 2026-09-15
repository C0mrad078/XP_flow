import { useEffect, useMemo, useState } from "react";
import {
  Activity as ActivityIcon,
  AlertTriangle,
  Film,
  Radio,
  Settings as SettingsIcon,
  Upload,
  XCircle,
} from "lucide-react";

import { PageContainer } from "@/components/common/page-container";
import { PageHeader } from "@/components/common/page-header";
import { EmptyState } from "@/components/feedback/empty-state";
import { ErrorState } from "@/components/feedback/error-state";
import { LoadingState } from "@/components/feedback/loading-state";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { mockActivityEvents } from "@/development/mock-data/activity";
import { activityApi } from "@/lib/tauri";
import { formatDateTime, formatTime } from "@/lib/formatting/date";
import { cn } from "@/lib/utilities/cn";
import type { ActivityCategory, ActivityEvent } from "@/types/domain";

const CATEGORY_ICON: Record<ActivityCategory, typeof ActivityIcon> = {
  system: SettingsIcon,
  content: Film,
  publication: Upload,
  platform: Radio,
  warning: AlertTriangle,
  error: XCircle,
};

const LEVEL_DOT_CLASS: Record<ActivityEvent["level"], string> = {
  info: "bg-muted-foreground",
  success: "bg-success",
  warning: "bg-warning",
  error: "bg-danger",
};

const CATEGORY_TABS: Array<{ value: "all" | ActivityCategory; label: string }> = [
  { value: "all", label: "All" },
  { value: "system", label: "System" },
  { value: "content", label: "Content" },
  { value: "publication", label: "Publication" },
  { value: "platform", label: "Platform" },
  { value: "warning", label: "Warning" },
  { value: "error", label: "Error" },
];

type LoadState = "loading" | "ready" | "error";

export function ActivityPage() {
  const [events, setEvents] = useState<ActivityEvent[]>([]);
  const [state, setState] = useState<LoadState>("loading");
  const [filter, setFilter] = useState<"all" | ActivityCategory>("all");

  function fetchActivity() {
    return activityApi
      .listRecent(200)
      .then((real) => {
        const merged = [...real, ...mockActivityEvents].sort(
          (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
        );
        setEvents(merged);
        setState("ready");
      })
      .catch(() => setState("error"));
  }

  function retry() {
    setState("loading");
    fetchActivity();
  }

  useEffect(() => {
    fetchActivity();
  }, []);

  const filtered = useMemo(
    () => (filter === "all" ? events : events.filter((e) => e.category === filter)),
    [events, filter],
  );

  return (
    <PageContainer>
      <PageHeader title="Activity" description="A local, real-time trail of everything XP FLOW has done." />

      <Tabs value={filter} onValueChange={(value) => setFilter(value as typeof filter)}>
        <TabsList>
          {CATEGORY_TABS.map((tab) => (
            <TabsTrigger key={tab.value} value={tab.value}>
              {tab.label}
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>

      {state === "loading" && <LoadingState label="Loading activity…" />}
      {state === "error" && <ErrorState onRetry={retry} />}
      {state === "ready" && filtered.length === 0 && (
        <EmptyState
          icon={ActivityIcon}
          title="No activity yet"
          description="Nothing has happened here yet — actions you take will show up in this log."
        />
      )}
      {state === "ready" && filtered.length > 0 && (
        <div className="flex flex-col divide-y divide-border rounded-lg border border-border">
          {filtered.map((event) => {
            const Icon = CATEGORY_ICON[event.category];
            return (
              <div key={event.id} className="flex items-start gap-3 px-4 py-3">
                <div className="flex flex-col items-center gap-1 pt-0.5">
                  <span className={cn("size-1.5 rounded-full", LEVEL_DOT_CLASS[event.level])} aria-hidden />
                </div>
                <Icon className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
                <div className="min-w-0 flex-1">
                  <p className="text-body-small text-foreground">{event.message}</p>
                  <p
                    className="text-caption normal-case tracking-normal"
                    title={formatDateTime(event.created_at)}
                  >
                    {formatTime(event.created_at)}
                  </p>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </PageContainer>
  );
}
