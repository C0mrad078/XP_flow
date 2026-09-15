import { BarChart3 } from "lucide-react";

import { PlaceholderPage } from "@/components/common/placeholder-page";

export function AnalyticsPage() {
  return (
    <PlaceholderPage
      title="Analytics"
      icon={BarChart3}
      description="Deep performance analytics synced from every connected platform."
      emptyTitle="Analytics synchronization is coming soon"
      emptyDescription="The Dashboard shows a Phase 1 performance overview with mock data; live metrics sync arrives once platform connectors are real."
    />
  );
}
