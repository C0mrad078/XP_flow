import { useState } from "react";
import { Calendar, LayoutGrid, ListPlus, List, Rows3, ShieldCheck } from "lucide-react";
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
import { useRecordPublicationConsent } from "@/hooks/use-publishing";
import { isFeatureEnabled } from "@/lib/utilities/feature-flags";
import { toast } from "@/stores/toast-store";
import { useWorkspaceStore } from "@/stores/workspace-store";
import type { Publication } from "@/types/domain";
import type { Platform } from "@/types/domain";
import { deriveConnectionHealth } from "@/types/platform-auth";

import { PublicationDetailsDrawer } from "./publication-details-drawer";
import { QueueListView } from "./queue-list-view";
import { QueueTimelineView } from "./queue-timeline-view";
import { BulkTikTokApprovalDialog } from "./tiktok-consent-dialog";

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
  const [search, setSearch] = useState("");
  const [platform, setPlatform] = useState<Platform | "">("");
  const [channelId, setChannelId] = useState("");
  const [accountId, setAccountId] = useState("");
  const [attentionOnly, setAttentionOnly] = useState(false);
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
    search: search || undefined,
    platform: platform || undefined,
    channel_id: channelId || undefined,
    platform_account_id: accountId || undefined,
    requires_attention: attentionOnly,
  });

  const channelNames = new Map(channels.map((c) => [c.id, c.name]));
  const publications: Publication[] = page?.items ?? [];

  // Section 44/47: a heuristic candidate list, not a confirmed one —
  // any TikTok publication about to become due, or already known to be
  // blocked on consent specifically, may need approval. Approving one
  // that already had valid consent is a harmless no-op (an extra audit
  // row), so a false positive here costs nothing; a false negative would
  // hide a real block, which this deliberately avoids by erring wide.
  const tiktokNeedingApproval = publications.filter(
    (p) =>
      p.platform === "tiktok" &&
      (p.status === "scheduled" || p.status === "queued" || p.last_error_code === "CONSENT_REQUIRED"),
  );
  const [bulkApprovalOpen, setBulkApprovalOpen] = useState(false);
  const bulkConsent = useRecordPublicationConsent();

  async function handleBulkApprove(selectedIds: string[]) {
    let failures = 0;
    for (const id of selectedIds) {
      try {
        await bulkConsent.mutateAsync({ id, source: "bulk_approval" });
      } catch {
        failures += 1;
      }
    }
    setBulkApprovalOpen(false);
    if (failures > 0) {
      toast({
        variant: "warning",
        title: "Some approvals failed",
        description: `${selectedIds.length - failures} of ${selectedIds.length} approved.`,
      });
    } else {
      toast({ variant: "success", title: `${selectedIds.length} TikTok publications approved` });
    }
  }

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
          <div className="flex items-center gap-2">
            {tiktokNeedingApproval.length > 0 && (
              <Button size="sm" variant="outline" onClick={() => setBulkApprovalOpen(true)}>
                <ShieldCheck className="size-3.5" />
                {tiktokNeedingApproval.length} TikTok approvals
              </Button>
            )}
            <Button size="sm" onClick={() => navigate("/content")}>
              <ListPlus />
              Add to queue
            </Button>
          </div>
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

      <div className="flex flex-wrap items-center gap-2 rounded-md border border-border bg-surface-elevated p-3">
        <input
          aria-label="Search publications"
          className="h-8 min-w-48 flex-1 rounded-md border border-border bg-surface px-2 text-body-small text-foreground"
          placeholder="Search title or remote ID"
          value={search}
          onChange={(event) => setSearch(event.target.value)}
        />
        <select
          aria-label="Filter platform"
          className="h-8 rounded-md border border-border bg-surface px-2 text-body-small"
          value={platform}
          onChange={(event) => setPlatform(event.target.value as Platform | "")}
        >
          <option value="">All platforms</option>
          <option value="youtube">YouTube</option>
          <option value="tiktok">TikTok</option>
          <option value="kwai">Kwai</option>
        </select>
        <select
          aria-label="Filter channel"
          className="h-8 rounded-md border border-border bg-surface px-2 text-body-small"
          value={channelId}
          onChange={(event) => setChannelId(event.target.value)}
        >
          <option value="">All channels</option>
          {channels.map((channel) => (
            <option key={channel.id} value={channel.id}>
              {channel.name}
            </option>
          ))}
        </select>
        <select
          aria-label="Filter account"
          className="h-8 rounded-md border border-border bg-surface px-2 text-body-small"
          value={accountId}
          onChange={(event) => setAccountId(event.target.value)}
        >
          <option value="">All accounts</option>
          {platformAccounts.map((account) => (
            <option key={account.id} value={account.id}>
              {account.display_name ?? account.username_or_handle ?? account.platform}
            </option>
          ))}
        </select>
        <label className="flex items-center gap-2 text-body-small text-muted-foreground">
          <input
            type="checkbox"
            checked={attentionOnly}
            onChange={(event) => setAttentionOnly(event.target.checked)}
          />{" "}
          Requires attention
        </label>
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

      <BulkTikTokApprovalDialog
        publications={tiktokNeedingApproval}
        open={bulkApprovalOpen}
        onOpenChange={setBulkApprovalOpen}
        onApprove={handleBulkApprove}
        isApproving={bulkConsent.isPending}
        channelNames={channelNames}
      />
    </PageContainer>
  );
}
