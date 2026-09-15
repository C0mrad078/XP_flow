import { AlertTriangle, CheckCircle2, Clock, Loader2, XCircle } from "lucide-react";

import { MediaThumbnail } from "@/components/common/media-thumbnail";
import { PageContainer } from "@/components/common/page-container";
import { PageHeader } from "@/components/common/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { PlatformBadge } from "@/components/ui/platform-badge";
import { getTodayQueueItems, type MockQueueItem } from "@/development/mock-data/queue";
import { formatTime } from "@/lib/formatting/date";
import type { Platform, PublicationStatus } from "@/types/domain";

const FAILURE_STATUSES: PublicationStatus[] = ["failed", "blocked"];
const ACTIVE_STATUSES: PublicationStatus[] = ["uploading", "processing"];
const WARNING_STATUSES: PublicationStatus[] = ["auth_required", "rate_limited", "paused", "duplicate"];

function statuses(item: MockQueueItem): PublicationStatus[] {
  return Object.values(item.platforms) as PublicationStatus[];
}

function classify(item: MockQueueItem): "failed" | "active" | "warning" | "published" | "remaining" {
  const values = statuses(item);
  if (values.some((s) => FAILURE_STATUSES.includes(s))) return "failed";
  if (values.some((s) => ACTIVE_STATUSES.includes(s))) return "active";
  if (values.some((s) => WARNING_STATUSES.includes(s))) return "warning";
  if (values.every((s) => s === "published")) return "published";
  return "remaining";
}

export function TodayPage() {
  const todayItems = getTodayQueueItems();
  const buckets = {
    published: todayItems.filter((i) => classify(i) === "published"),
    remaining: todayItems.filter((i) => classify(i) === "remaining"),
    failed: todayItems.filter((i) => classify(i) === "failed"),
    active: todayItems.filter((i) => classify(i) === "active"),
    warning: todayItems.filter((i) => classify(i) === "warning"),
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
        <StatTile icon={XCircle} tone="danger" label="Failures" value={buckets.failed.length} />
      </div>

      {buckets.warning.length > 0 && (
        <Card className="border-warning/30 bg-warning/[0.04]">
          <CardHeader className="flex-row items-center gap-2 space-y-0">
            <AlertTriangle className="size-4 text-warning" />
            <CardTitle className="text-warning">Operational warnings</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-3">
            {buckets.warning.map((item) => (
              <QueueRow key={item.id} item={item} />
            ))}
          </CardContent>
        </Card>
      )}

      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">
        <QueueSection
          title="Currently uploading"
          items={buckets.active}
          emptyLabel="Nothing uploading right now"
        />
        <QueueSection title="Failures" items={buckets.failed} emptyLabel="No failures today" />
        <QueueSection title="Published today" items={buckets.published} emptyLabel="Nothing published yet" />
        <QueueSection
          title="Remaining today"
          items={buckets.remaining}
          emptyLabel="Queue is clear for today"
        />
      </div>
    </PageContainer>
  );
}

function QueueSection({
  title,
  items,
  emptyLabel,
}: {
  title: string;
  items: MockQueueItem[];
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
              <QueueRow key={item.id} item={item} />
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function QueueRow({ item }: { item: MockQueueItem }) {
  return (
    <div className="flex items-center gap-3">
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
