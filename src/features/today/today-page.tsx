import { AlertTriangle, CheckCircle2, Clock, Loader2, XCircle } from "lucide-react";

import { MediaThumbnail } from "@/components/common/media-thumbnail";
import { PageContainer } from "@/components/common/page-container";
import { PageHeader } from "@/components/common/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { PlatformBadge } from "@/components/ui/platform-badge";
import { useChannels } from "@/hooks/use-channels";
import { useCalendarRange, useDuePublications } from "@/hooks/use-scheduler";
import { formatTimeInZone, localDateKeyInZone } from "@/lib/formatting/date";
import { useWorkspaceStore } from "@/stores/workspace-store";
import type { CalendarPublication } from "@/types/scheduling";
import type { PublicationStatus } from "@/types/domain";

const FAILURE_STATUSES: PublicationStatus[] = ["failed", "blocked"];
const ACTIVE_STATUSES: PublicationStatus[] = ["uploading", "processing"];
const WARNING_STATUSES: PublicationStatus[] = ["auth_required", "rate_limited", "paused", "duplicate"];

function classify(item: CalendarPublication): "failed" | "active" | "warning" | "published" | "remaining" {
  const status = item.publication.status;
  if (FAILURE_STATUSES.includes(status)) return "failed";
  if (ACTIVE_STATUSES.includes(status)) return "active";
  if (WARNING_STATUSES.includes(status)) return "warning";
  if (status === "published") return "published";
  return "remaining";
}

export function TodayPage() {
  const timezone = useWorkspaceStore((state) => state.workspace?.timezone ?? "UTC");
  const workspaceId = useWorkspaceStore((state) => state.workspace?.id ?? null);
  const todayKey = localDateKeyInZone(new Date().toISOString(), timezone);
  const { data: calendarItems = [] } = useCalendarRange(null, todayKey, todayKey);
  const { data: overdue = [] } = useDuePublications();
  const { data: channels = [] } = useChannels();
  const channelNames = new Map(channels.map((c) => [c.id, c.name]));

  const buckets = {
    published: calendarItems.filter((i) => classify(i) === "published"),
    remaining: calendarItems.filter((i) => classify(i) === "remaining"),
    failed: calendarItems.filter((i) => classify(i) === "failed"),
    active: calendarItems.filter((i) => classify(i) === "active"),
    warning: calendarItems.filter((i) => classify(i) === "warning"),
  };

  return (
    <PageContainer>
      <PageHeader
        title="Today"
        description="What's published, what's next, and what needs attention right now."
      />

      <div className="grid grid-cols-2 gap-4 lg:grid-cols-4">
        <StatTile
          icon={CheckCircle2}
          tone="success"
          label="Published today"
          value={buckets.published.length}
        />
        <StatTile icon={Clock} tone="neutral" label="Remaining today" value={buckets.remaining.length} />
        <StatTile
          icon={Loader2}
          tone="primary"
          label="Currently uploading"
          value={buckets.active.length}
          spin
        />
        <StatTile
          icon={XCircle}
          tone="danger"
          label={workspaceId ? "Overdue" : "Failures"}
          value={workspaceId ? overdue.length : buckets.failed.length}
        />
      </div>

      {(buckets.warning.length > 0 || overdue.length > 0) && (
        <Card className="border-warning/30 bg-warning/[0.04]">
          <CardHeader className="flex-row items-center gap-2 space-y-0">
            <AlertTriangle className="size-4 text-warning" />
            <CardTitle className="text-warning">Operational warnings</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-3">
            {overdue.map((publication) => (
              <div key={publication.id} className="flex items-center gap-3">
                <MediaThumbnail seed={publication.video_id} className="h-12 w-8" />
                <div className="min-w-0 flex-1">
                  <p className="truncate text-body-small font-medium text-foreground">{publication.title}</p>
                  <p className="text-caption normal-case tracking-normal text-danger">Overdue</p>
                </div>
                <PlatformBadge platform={publication.platform} size="sm" iconOnly />
              </div>
            ))}
            {buckets.warning.map((item) => (
              <QueueRow
                key={item.publication.id}
                item={item}
                channelName={channelNames.get(item.publication.channel_id)}
                timezone={timezone}
              />
            ))}
          </CardContent>
        </Card>
      )}

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <QueueSection
          title="Currently uploading"
          items={buckets.active}
          channelNames={channelNames}
          timezone={timezone}
          emptyLabel="Nothing uploading right now"
        />
        <QueueSection
          title="Failures"
          items={buckets.failed}
          channelNames={channelNames}
          timezone={timezone}
          emptyLabel="No failures today"
        />
        <QueueSection
          title="Published today"
          items={buckets.published}
          channelNames={channelNames}
          timezone={timezone}
          emptyLabel="Nothing published yet"
        />
        <QueueSection
          title="Remaining today"
          items={buckets.remaining}
          channelNames={channelNames}
          timezone={timezone}
          emptyLabel="Queue is clear for today"
        />
      </div>
    </PageContainer>
  );
}

function QueueSection({
  title,
  items,
  channelNames,
  timezone,
  emptyLabel,
}: {
  title: string;
  items: CalendarPublication[];
  channelNames: Map<string, string>;
  timezone: string;
  emptyLabel: string;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>
          {title} <span className="text-muted-foreground">({items.length})</span>
        </CardTitle>
      </CardHeader>
      <CardContent>
        {items.length === 0 ? (
          <p className="text-body-small py-4 text-center text-muted-foreground">{emptyLabel}</p>
        ) : (
          <div className="flex flex-col gap-3">
            {items.map((item) => (
              <QueueRow
                key={item.publication.id}
                item={item}
                channelName={channelNames.get(item.publication.channel_id)}
                timezone={timezone}
              />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function QueueRow({
  item,
  channelName,
  timezone,
}: {
  item: CalendarPublication;
  channelName?: string;
  timezone: string;
}) {
  const { publication } = item;
  return (
    <div className="flex items-center gap-3">
      <MediaThumbnail seed={publication.video_id} className="h-12 w-8" />
      <div className="min-w-0 flex-1">
        <p className="truncate text-body-small font-medium text-foreground">{publication.title}</p>
        <p className="text-caption normal-case tracking-normal">{channelName ?? "Unknown channel"}</p>
      </div>
      <PlatformBadge platform={publication.platform} size="sm" iconOnly />
      <span className="w-14 shrink-0 text-right font-mono-data text-body-small text-muted-foreground">
        {publication.scheduled_at ? formatTimeInZone(publication.scheduled_at, timezone) : "—"}
      </span>
    </div>
  );
}

function StatTile({
  icon: Icon,
  tone,
  label,
  value,
  spin,
}: {
  icon: typeof CheckCircle2;
  tone: "success" | "neutral" | "primary" | "danger";
  label: string;
  value: number;
  spin?: boolean;
}) {
  const toneClass = {
    success: "text-success bg-success/10",
    neutral: "text-muted-foreground bg-surface-elevated",
    primary: "text-primary bg-primary/10",
    danger: "text-danger bg-danger/10",
  }[tone];

  return (
    <Card>
      <CardContent className="flex items-center gap-3 p-5">
        <div className={`flex size-9 items-center justify-center rounded-lg ${toneClass}`}>
          <Icon className={`size-4 ${spin && value > 0 ? "animate-spin" : ""}`} />
        </div>
        <div>
          <p className="text-metric-small text-foreground">{value}</p>
          <p className="text-caption">{label}</p>
        </div>
      </CardContent>
    </Card>
  );
}
