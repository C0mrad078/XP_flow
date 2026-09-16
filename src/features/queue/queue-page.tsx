import { useState } from "react";
import { Calendar, LayoutGrid, ListPlus, List, Rows3 } from "lucide-react";
import { useNavigate } from "react-router-dom";

import { PageContainer } from "@/components/common/page-container";
import { PageHeader } from "@/components/common/page-header";
import { EmptyState } from "@/components/feedback/empty-state";
import { ErrorState } from "@/components/feedback/error-state";
import { LoadingState } from "@/components/feedback/loading-state";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useChannels } from "@/hooks/use-channels";
import { usePlatformAccountsForWorkspace } from "@/hooks/use-platform-accounts";
import { useQueueList } from "@/hooks/use-queue";
import { isFeatureEnabled } from "@/lib/utilities/feature-flags";
import { useWorkspaceStore } from "@/stores/workspace-store";
import type { Publication } from "@/types/domain";
import { deriveConnectionHealth } from "@/types/platform-auth";

import { PublicationDetailsDrawer } from "./publication-details-drawer";
import { QueueListView } from "./queue-list-view";
import { QueueTimelineView } from "./queue-timeline-view";

type QueueViewMode = "timeline" | "list";

/** Future view modes the Queue screen is architected for but does not
 * implement yet — shown disabled so the eventual UI surface is visible
 * without pretending the mode already works. */
const FUTURE_VIEW_MODES = [
  { id: "compact", label: "Compact", icon: Rows3 },
  { id: "kanban", label: "Kanban", icon: LayoutGrid },
];

export function QueuePage() {
  const [viewMode, setViewMode] = useState<QueueViewMode>("timeline");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const navigate = useNavigate();
  const timezone = useWorkspaceStore((state) => state.workspace?.timezone ?? "UTC");

  const { data: channels = [] } = useChannels();
  const { data: platformAccounts = [] } = usePlatformAccountsForWorkspace();
  const {
    data: page,
    isLoading,
    isError,
    refetch,
  } = useQueueList({
    statuses: [
      "queued",
      "scheduled",
      "uploading",
      "processing",
      "published",
      "failed",
      "retry_wait",
      "rate_limited",
      "auth_required",
      "blocked",
      "paused",
    ],
    page_size: 200,
  });

  const channelNames = new Map(channels.map((c) => [c.id, c.name]));
  const publications: Publication[] = page?.items ?? [];

  /** Section 72-76: informational only — a missing/unhealthy account never
   * removes or blocks a queued publication here, it only surfaces the
   * problem on the row via `QueueItemCard`'s tooltip. */
  function isAccountConnected(publication: Publication): boolean {
    return platformAccounts.some((account) => {
      if (account.channel_id !== publication.channel_id || account.platform !== publication.platform)
        return false;
      const health = deriveConnectionHealth(account);
      return health === "healthy" || health === "token_expiring";
    });
  }

  return (
    <PageContainer>
      <PageHeader
        title="Queue"
        description="Every publication scheduled to go out, across every channel and platform."
        actions={
          <Button size="sm" onClick={() => navigate("/content")}>
            <ListPlus />
            Add to queue
          </Button>
        }
      />

      <div className="flex items-center justify-between">
        <Tabs value={viewMode} onValueChange={(value) => setViewMode(value as QueueViewMode)}>
          <TabsList>
            <TabsTrigger value="timeline">
              <Rows3 className="mr-1.5 size-3.5" />
              Timeline
            </TabsTrigger>
            <TabsTrigger value="list">
              <List className="mr-1.5 size-3.5" />
              List
            </TabsTrigger>
          </TabsList>
        </Tabs>

        <div className="flex items-center gap-1">
          <Tooltip>
            <TooltipTrigger asChild>
              <span>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={() => navigate("/calendar")}
                  aria-label="Calendar view"
                >
                  <Calendar className="size-4" />
                </Button>
              </span>
            </TooltipTrigger>
            <TooltipContent>Open Calendar</TooltipContent>
          </Tooltip>
          {!isFeatureEnabled("queueKanbanView") &&
            FUTURE_VIEW_MODES.map((mode) => (
              <Tooltip key={mode.id}>
                <TooltipTrigger asChild>
                  <span>
                    <Button
                      variant="ghost"
                      size="icon"
                      disabled
                      aria-label={`${mode.label} view (coming soon)`}
                    >
                      <mode.icon className="size-4" />
                    </Button>
                  </span>
                </TooltipTrigger>
                <TooltipContent>{mode.label} view — coming soon</TooltipContent>
              </Tooltip>
            ))}
        </div>
      </div>

      {isLoading && <LoadingState label="Loading queue…" />}
      {isError && <ErrorState onRetry={() => refetch()} />}

      {page && publications.length === 0 && (
        <EmptyState
          icon={Rows3}
          title="Queue is empty"
          description="Import videos from the Content library and add them to the queue to schedule publications."
          action={
            <Button size="sm" onClick={() => navigate("/content")}>
              <ListPlus />
              Add to queue
            </Button>
          }
        />
      )}

      {page &&
        publications.length > 0 &&
        (viewMode === "timeline" ? (
          <QueueTimelineView
            publications={publications}
            channelNames={channelNames}
            timezone={timezone}
            onSelect={(p) => setSelectedId(p.id)}
            isAccountConnected={isAccountConnected}
          />
        ) : (
          <QueueListView
            publications={publications}
            channelNames={channelNames}
            timezone={timezone}
            onSelect={(p) => setSelectedId(p.id)}
            isAccountConnected={isAccountConnected}
          />
        ))}

      <PublicationDetailsDrawer publicationId={selectedId} onClose={() => setSelectedId(null)} />
    </PageContainer>
  );
}
