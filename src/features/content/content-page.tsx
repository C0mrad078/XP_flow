import { useMemo, useState } from "react";
import { Film, LayoutGrid, List, SlidersHorizontal, Upload } from "lucide-react";

import { PageContainer } from "@/components/common/page-container";
import { PageHeader } from "@/components/common/page-header";
import { EmptyState } from "@/components/feedback/empty-state";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { mockContentItems } from "@/development/mock-data/content";

import { ContentItemCard } from "./content-item-card";
import { ContentListHeader, ContentListRow } from "./content-list-row";

type ContentViewMode = "grid" | "list";

export function ContentPage() {
  const [viewMode, setViewMode] = useState<ContentViewMode>("grid");
  const [query, setQuery] = useState("");

  const filtered = useMemo(() => {
    const trimmed = query.trim().toLowerCase();
    if (!trimmed) return mockContentItems;
    return mockContentItems.filter((item) => item.title.toLowerCase().includes(trimmed));
  }, [query]);

  return (
    <PageContainer>
      <PageHeader
        title="Content"
        description="Every video imported into XP FLOW, ready to be scheduled for publication."
        actions={
          <Button size="sm">
            <Upload />
            Import video
          </Button>
        }
      />

      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <Input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search content…"
            className="w-64"
          />
          <Button variant="outline" size="sm" disabled title="Filtering is coming soon">
            <SlidersHorizontal />
            Filters
          </Button>
        </div>

        <Tabs value={viewMode} onValueChange={(value) => setViewMode(value as ContentViewMode)}>
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

      {filtered.length === 0 ? (
        <EmptyState
          icon={Film}
          title="No videos yet"
          description="Import a video from your local library or a Cut.pro export to get started."
        />
      ) : viewMode === "grid" ? (
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
          {filtered.map((item) => (
            <ContentItemCard key={item.id} item={item} />
          ))}
        </div>
      ) : (
        <div>
          <ContentListHeader />
          <div className="flex flex-col divide-y divide-border">
            {filtered.map((item) => (
              <ContentListRow key={item.id} item={item} />
            ))}
          </div>
        </div>
      )}
    </PageContainer>
  );
}
