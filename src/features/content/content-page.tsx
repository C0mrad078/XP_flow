import { useState } from "react";
import { Film, FolderPlus, LayoutGrid, List } from "lucide-react";

import { PageContainer } from "@/components/common/page-container";
import { PageHeader } from "@/components/common/page-header";
import { EmptyState } from "@/components/feedback/empty-state";
import { ErrorState } from "@/components/feedback/error-state";
import { LoadingState } from "@/components/feedback/loading-state";
import { Button } from "@/components/ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useChannels } from "@/hooks/use-channels";
import { useContentLibrary } from "@/hooks/use-content";
import { useSources } from "@/hooks/use-sources";
import { useContentStore } from "@/stores/content-store";
import type { VideoSort } from "@/types/media";

import { AddFolderDialog } from "./add-folder-dialog";
import { BulkActionBar } from "./bulk-action-bar";
import { ContentDetailsDrawer } from "./content-details-drawer";
import { ContentFilters } from "./content-filters";
import { ContentSummaryBar } from "./content-summary-bar";
import { DropZoneOverlay } from "./drop-zone-overlay";
import { ImportMenu } from "./import-menu";
import { PaginationControls } from "./pagination-controls";
import { QuickPreviewOverlay } from "./quick-preview-overlay";
import { useFileDrop } from "./use-file-drop";
import { useQuickPreviewKeys } from "./use-quick-preview-keys";
import { VideoCard } from "./video-card";
import { VideoListHeader, VideoListRow } from "./video-list-row";

const SORT_LABELS: Record<VideoSort, string> = {
  newest_imported: "Newest imported",
  oldest_imported: "Oldest imported",
  filename: "Filename",
  duration: "Duration",
  file_size: "File size",
  channel: "Channel",
};

export function ContentPage() {
  const { data: page, isLoading, isError, refetch } = useContentLibrary();
  const { data: channels = [] } = useChannels();
  const { data: sources = [] } = useSources();
  const viewMode = useContentStore((state) => state.viewMode);
  const setViewMode = useContentStore((state) => state.setViewMode);
  const filters = useContentStore((state) => state.filters);
  const setFilters = useContentStore((state) => state.setFilters);
  const selectedIds = useContentStore((state) => state.selectedIds);
  const toggleSelected = useContentStore((state) => state.toggleSelected);

  const { isDraggingOver } = useFileDrop();
  useQuickPreviewKeys();

  const [addFolderOpen, setAddFolderOpen] = useState(false);

  const channelName = (id: string | null) => (id ? channels.find((c) => c.id === id)?.name : undefined);
  const sourceName = (id: string) => sources.find((s) => s.id === id)?.name;

  return (
    <PageContainer>
      <PageHeader
        title="Content"
        description="Every video XP FLOW has indexed — imported manually or ingested automatically from a watched folder."
        actions={
          <>
            <Button variant="outline" size="sm" onClick={() => setAddFolderOpen(true)}>
              <FolderPlus />
              Add Folder
            </Button>
            <ImportMenu />
          </>
        }
      />

      <ContentSummaryBar />

      <div className="flex flex-wrap items-center justify-between gap-3">
        <ContentFilters />

        <div className="flex items-center gap-2">
          <Select
            value={filters.sort}
            onValueChange={(value) => setFilters({ sort: value as VideoSort, page: filters.page })}
          >
            <SelectTrigger className="h-9 w-44">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {(Object.keys(SORT_LABELS) as VideoSort[]).map((sort) => (
                <SelectItem key={sort} value={sort}>
                  {SORT_LABELS[sort]}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          <Tabs value={viewMode} onValueChange={(value) => setViewMode(value as typeof viewMode)}>
            <TabsList>
              <TabsTrigger value="grid">
                <LayoutGrid className="mr-1.5 size-3.5" />
                Grid
              </TabsTrigger>
              <TabsTrigger value="list">
                <List className="mr-1.5 size-3.5" />
                List
              </TabsTrigger>
            </TabsList>
          </Tabs>
        </div>
      </div>

      {isLoading && <LoadingState label="Loading content library…" />}
      {isError && <ErrorState onRetry={() => refetch()} />}

      {page && page.items.length === 0 && (
        <EmptyState
          icon={Film}
          title="No videos yet"
          description="Import a video, drag files in, or add a folder to start building your library."
          action={
            <Button size="sm" onClick={() => setAddFolderOpen(true)}>
              <FolderPlus />
              Add Folder
            </Button>
          }
        />
      )}

      {page && page.items.length > 0 && (
        <>
          {viewMode === "grid" ? (
            <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5 xl:grid-cols-6">
              {page.items.map((video) => (
                <VideoCard
                  key={video.id}
                  video={video}
                  channelName={channelName(video.channel_id)}
                  selected={selectedIds.includes(video.id)}
                  onToggleSelect={toggleSelected}
                />
              ))}
            </div>
          ) : (
            <div>
              <VideoListHeader />
              <div className="flex flex-col divide-y divide-border">
                {page.items.map((video) => (
                  <VideoListRow
                    key={video.id}
                    video={video}
                    channelName={channelName(video.channel_id)}
                    sourceName={sourceName(video.source_id)}
                    selected={selectedIds.includes(video.id)}
                    onToggleSelect={toggleSelected}
                  />
                ))}
              </div>
            </div>
          )}

          <PaginationControls
            page={page.page}
            pageSize={page.page_size}
            total={page.total}
            onPageChange={(p) => setFilters({ page: p })}
          />
        </>
      )}

      <BulkActionBar />
      <ContentDetailsDrawer />
      <QuickPreviewOverlay />
      <DropZoneOverlay visible={isDraggingOver} />
      <AddFolderDialog open={addFolderOpen} onOpenChange={setAddFolderOpen} />
    </PageContainer>
  );
}
