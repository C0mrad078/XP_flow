import { Workflow } from "lucide-react";

import { PlaceholderPage } from "@/components/common/placeholder-page";

export function AutomationPage() {
  return (
    <PlaceholderPage
      title="Automation"
      icon={Workflow}
      description="Rules that react to publication events — retries, alerts, and cross-platform actions."
      emptyTitle="Automation rules are coming soon"
      emptyDescription="The Job foundation (job types/statuses) exists on the backend; the rule builder and scheduler ship in a later phase."
    />
  );
}
