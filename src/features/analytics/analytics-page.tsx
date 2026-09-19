import { BarChart3, RefreshCcw } from "lucide-react";
import { useState } from "react";
import { PageContainer } from "@/components/common/page-container";
import { PageHeader } from "@/components/common/page-header";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { EmptyState } from "@/components/feedback/empty-state";
import { Button } from "@/components/ui/button";
import { useQueueList } from "@/hooks/use-queue";
import {
  useAnalyticsCapabilities,
  usePublicationAnalytics,
  useSyncPublicationAnalytics,
  useSyncWorkspaceAnalytics,
} from "@/hooks/use-analytics";
import type { Platform, Publication } from "@/types/domain";

function PublishedRow({ publication, days }: { publication: Publication; days: number }) {
  const metrics = usePublicationAnalytics(publication.id, days);
  const sync = useSyncPublicationAnalytics();
  const latest = metrics.data?.[metrics.data.length - 1];
  return (
    <div className="flex items-center justify-between gap-3 border-b border-border py-3 last:border-0">
      <div className="min-w-0">
        <p className="truncate text-body-small font-medium text-foreground">{publication.title}</p>
        <p className="text-caption text-muted-foreground">
          {publication.platform} ·{" "}
          {latest ? `Updated ${new Date(latest.captured_at).toLocaleString()}` : "No provider snapshot yet"}
        </p>
      </div>
      <div className="flex items-center gap-3">
        <span className="font-mono-data text-body-small text-foreground">
          {latest?.views == null ? "Unavailable" : `${latest.views.toLocaleString()} views`}
        </span>
        <Button
          variant="ghost"
          size="sm"
          disabled={sync.isPending}
          onClick={() => sync.mutate(publication.id)}
        >
          {sync.isPending ? "Refreshing…" : "Refresh"}
        </Button>
      </div>
    </div>
  );
}

export function AnalyticsPage() {
  const { data: page, isLoading } = useQueueList({
    statuses: ["published"],
    sort: "newest_first",
    page_size: 20,
  });
  const platform = (page?.items?.[0]?.platform ?? "youtube") as Platform;
  const capabilities = useAnalyticsCapabilities(platform);
  const workspaceSync = useSyncWorkspaceAnalytics();
  const [days, setDays] = useState(30);
  const publications = page?.items ?? [];
  return (
    <PageContainer>
      <PageHeader
        title="Analytics"
        description="Persisted provider metrics and XP FLOW publication performance."
        actions={
          <div className="flex items-center gap-2">
            <select
              aria-label="Analytics range"
              className="h-8 rounded-md border border-border bg-background px-2 text-body-small"
              value={days}
              onChange={(event) => setDays(Number(event.target.value))}
            >
              <option value={7}>7 days</option>
              <option value={30}>30 days</option>
              <option value={90}>90 days</option>
            </select>
            <Button
              variant="outline"
              size="sm"
              onClick={() => workspaceSync.mutate()}
              disabled={workspaceSync.isPending}
            >
              <RefreshCcw className="size-3.5" />
              {workspaceSync.isPending ? "Refreshing…" : "Refresh"}
            </Button>
          </div>
        }
      />
      <Card>
        <CardHeader>
          <CardTitle>Provider capabilities</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-body-small text-muted-foreground">
            {capabilities.data
              ? `${platform} supports ${[capabilities.data.publication_views && "views", capabilities.data.publication_likes && "likes", capabilities.data.publication_comments && "comments"].filter(Boolean).join(", ") || "no publication metrics"}.`
              : "Loading capability information…"}
          </p>
        </CardContent>
      </Card>
      {isLoading && <p className="text-body-small text-muted-foreground">Loading published content…</p>}
      {!isLoading && publications.length === 0 && (
        <EmptyState
          icon={BarChart3}
          title="No published content"
          description="Analytics will appear after a provider publication has been completed and synchronized."
        />
      )}
      {publications.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Recent publications</CardTitle>
          </CardHeader>
          <CardContent>
            {publications.map((publication) => (
              <PublishedRow key={publication.id} publication={publication} days={days} />
            ))}
          </CardContent>
        </Card>
      )}
    </PageContainer>
  );
}
