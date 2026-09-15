import { Calendar } from "lucide-react";

import { PlaceholderPage } from "@/components/common/placeholder-page";

export function CalendarPage() {
  return (
    <PlaceholderPage
      title="Calendar"
      icon={Calendar}
      description="A month/week view of every scheduled publication across channels and platforms."
      emptyTitle="Calendar view is coming soon"
      emptyDescription="Phase 1 ships the Queue's Timeline and List views; the full calendar view mode lands in a later phase."
    />
  );
}
