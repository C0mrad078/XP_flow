import { MessageSquare } from "lucide-react";

import { PlaceholderPage } from "@/components/common/placeholder-page";

export function CommentsPage() {
  return (
    <PlaceholderPage
      title="Comments"
      icon={MessageSquare}
      description="Cross-platform comment moderation and reply automation for every connected channel."
      emptyTitle="Comment management is coming soon"
      emptyDescription="This requires real platform connectors, which are stubbed in Phase 1 (see the Channels screen) but not yet implemented."
    />
  );
}
