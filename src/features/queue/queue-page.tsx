import { useState } from "react";
import { Calendar, LayoutGrid, List, Plus, Rows3 } from "lucide-react";

import { PageContainer } from "@/components/common/page-container";
import { PageHeader } from "@/components/common/page-header";
import { EmptyState } from "@/components/feedback/empty-state";
import { Button } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { mockQueueItems } from "@/development/mock-data/queue";
import { isFeatureEnabled } from "@/lib/utilities/feature-flags";

import { QueueListView } from "./queue-list-view";
import { QueueTimelineView } from "./queue-timeline-view";

type QueueViewMode = "timeline" | "list";

/** Future view modes the Queue screen is architected for (section 27) but
 * does not implement yet — shown disabled so the eventual UI surface is
 * visible without pretending the mode already works. */
const FUTURE_VIEW_MODES = [
  { id: "compact", label: "Compact", icon: Rows3 },
  { id: "calendar", label: "Calendar", icon: Calendar },
  { id: "kanban", label: "Kanban", icon: LayoutGrid },
];

export function QueuePage() {
  const [viewMode, setViewMode] = useState<QueueViewMode>("timeline");

  return (
    <PageContainer>
      <PageHeader
        title="Queue"
        description="Every publication scheduled to go out, across every channel and platform."
        actions={
          <Button size="sm">
            <Plus />
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

        {!isFeatureEnabled("queueKanbanView") && (
          <div className="flex items-center gap-1">
            {FUTURE_VIEW_MODES.map((mode) => (
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
        )}
      </div>

      {mockQueueItems.length === 0 ? (
        <EmptyState
          icon={Rows3}
          title="Queue is empty"
          description="Import videos from the Content library and add them to the queue to schedule publications."
        />
      ) : viewMode === "timeline" ? (
        <QueueTimelineView items={mockQueueItems} />
      ) : (
        <QueueListView items={mockQueueItems} />
      )}
    </PageContainer>
  );
}
