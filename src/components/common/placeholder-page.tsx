import type { LucideIcon } from "lucide-react";

import { PageContainer } from "@/components/common/page-container";
import { PageHeader } from "@/components/common/page-header";
import { EmptyState } from "@/components/feedback/empty-state";
import { Badge } from "@/components/ui/badge";

export interface PlaceholderPageProps {
  title: string;
  icon: LucideIcon;
  description: string;
  emptyTitle: string;
  emptyDescription: string;
}

/** Renders a screen this brief intentionally defers past Phase 1 (Calendar,
 * Comments, Analytics, Automation — section 57). Real architecture (routes,
 * nav entry, domain tables where relevant) exists; only the feature UI is
 * deferred, and that is stated on the page instead of faked. */
export function PlaceholderPage({
  title,
  icon: Icon,
  description,
  emptyTitle,
  emptyDescription,
}: PlaceholderPageProps) {
  return (
    <PageContainer>
      <PageHeader
        title={title}
        description={description}
        actions={
          <Badge variant="outline" className="gap-1.5">
            <Icon className="size-3" />
            Coming in a future phase
          </Badge>
        }
      />
      <div className="rounded-lg border border-dashed border-border">
        <EmptyState icon={Icon} title={emptyTitle} description={emptyDescription} />
      </div>
    </PageContainer>
  );
}
