import { useEffect, useState } from "react";
import { CalendarClock, Eye, ListVideo, Radio, TrendingUp } from "lucide-react";

import { PageContainer } from "@/components/common/page-container";
import { PageHeader } from "@/components/common/page-header";
import { MediaThumbnail } from "@/components/common/media-thumbnail";
import { SparklineChart } from "@/components/common/sparkline-chart";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Metric } from "@/components/ui/metric";
import { PlatformBadge } from "@/components/ui/platform-badge";
import { activityApi } from "@/lib/tauri";
import { formatCompactNumber, formatPercent } from "@/lib/formatting/number";
import { formatRelativeTime, formatTime } from "@/lib/formatting/date";
import { mockActivityEvents } from "@/development/mock-data/activity";
import {
  mockDashboardMetrics,
  mockPlatformOverview,
  mockViewsSeries,
} from "@/development/mock-data/dashboard";
import { mockQueueItems } from "@/development/mock-data/queue";
import type { ActivityEvent, Platform } from "@/types/domain";

const upcomingPublications = mockQueueItems
  .filter((item) => new Date(item.scheduledAt).getTime() > Date.now())
  .sort((a, b) => new Date(a.scheduledAt).getTime() - new Date(b.scheduledAt).getTime())
  .slice(0, 4);

export function DashboardPage() {
  const [activity, setActivity] = useState<ActivityEvent[]>(mockActivityEvents);

  useEffect(() => {
    let cancelled = false;
    activityApi
      .listRecent(10)
      .then((events) => {
        if (cancelled) return;
        const merged = [...events, ...mockActivityEvents]
          .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
          .slice(0, 6);
        setActivity(merged);
      })
      .catch(() => setActivity(mockActivityEvents.slice(0, 6)));
    return () => {
      cancelled = true;
    };
  }, []);

  const metrics = mockDashboardMetrics;

  return (
    <PageContainer>
      <PageHeader title="Dashboard" description="Everything happening across your channels, at a glance." />

      <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
        <Card>
          <CardContent className="p-5">
            <Metric
              label="Total views"
              value={formatCompactNumber(metrics.totalViews)}
              icon={Eye}
              trend={{ direction: "up", value: formatPercent(metrics.totalViewsTrend) }}
            />
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-5">
            <Metric
              label="Published today"
              value={`${metrics.publishedToday}/${metrics.publishedTodayTarget}`}
              icon={CalendarClock}
            />
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-5">
            <Metric label="Queued posts" value={String(metrics.queuedPosts)} icon={ListVideo} />
          </CardContent>
        </Card>
        <Card>
          <CardContent className="p-5">
            <Metric
              label="Active channels"
              value={`${metrics.activeChannels}/${metrics.totalChannels}`}
              icon={Radio}
            />
          </CardContent>
        </Card>
      </div>

      <div className="grid grid-cols-1 gap-4 xl:grid-cols-3">
        <Card className="xl:col-span-2">
          <CardHeader className="flex-row items-center justify-between">
            <CardTitle>Performance</CardTitle>
            <span className="inline-flex items-center gap-1 text-xs font-medium text-success">
              <TrendingUp className="size-3.5" />
              Last 14 days
            </span>
          </CardHeader>
          <CardContent>
            <SparklineChart values={mockViewsSeries} className="h-40 w-full" />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Platform overview</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-4">
            {mockPlatformOverview.map((row) => (
              <div key={row.platform} className="flex flex-col gap-1.5">
                <div className="flex items-center justify-between">
                  <PlatformBadge platform={row.platform} size="sm" />
                  <span className="font-mono-data text-body-small text-foreground">
                    {formatCompactNumber(row.views)}
                  </span>
                </div>
                <div className="h-1.5 overflow-hidden rounded-full bg-surface-hover">
                  <div className="h-full rounded-full bg-primary" style={{ width: `${row.share}%` }} />
                </div>
              </div>
            ))}
          </CardContent>
        </Card>
      </div>

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Next publications</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-3">
            {upcomingPublications.map((item) => (
              <div key={item.id} className="flex items-center gap-3">
                <MediaThumbnail seed={item.id} durationSeconds={item.durationSeconds} className="h-12 w-8" />
                <div className="min-w-0 flex-1">
                  <p className="truncate text-body-small font-medium text-foreground">{item.videoTitle}</p>
                  <p className="text-caption normal-case tracking-normal">{item.channelName}</p>
                </div>
                <div className="flex items-center gap-1">
                  {(Object.keys(item.platforms) as Platform[]).map((platform) => (
                    <PlatformBadge key={platform} platform={platform} size="sm" iconOnly />
                  ))}
                </div>
                <span className="w-14 shrink-0 text-right font-mono-data text-body-small text-muted-foreground">
                  {formatTime(item.scheduledAt)}
                </span>
              </div>
            ))}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Recent activity</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-3">
            {activity.map((event) => (
              <div key={event.id} className="flex items-start gap-3">
                <span className="mt-1.5 size-1.5 shrink-0 rounded-full bg-muted-foreground" aria-hidden />
                <div className="min-w-0 flex-1">
                  <p className="truncate text-body-small text-foreground">{event.message}</p>
                  <p className="text-caption normal-case tracking-normal">
                    {formatRelativeTime(event.created_at)}
                  </p>
                </div>
              </div>
            ))}
          </CardContent>
        </Card>
      </div>
    </PageContainer>
  );
}
