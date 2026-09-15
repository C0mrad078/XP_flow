import type * as React from "react";
import type { LucideIcon } from "lucide-react";

import { cn } from "@/lib/utilities/cn";

export interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description?: string;
  action?: React.ReactNode;
  className?: string;
}

/** Every major screen's "nothing here yet" state (section 39) — always
 * explains what the empty state means and what to do about it. */
export function EmptyState({ icon: Icon, title, description, action, className }: EmptyStateProps) {
  return (
    <div className={cn("flex flex-col items-center justify-center gap-3 px-6 py-16 text-center", className)}>
      <div className="flex size-12 items-center justify-center rounded-xl border border-border bg-surface-elevated">
        <Icon className="size-5 text-muted-foreground" />
      </div>
      <div className="flex flex-col gap-1">
        <p className="text-section-title text-foreground">{title}</p>
        {description && <p className="max-w-sm text-body-small text-muted-foreground">{description}</p>}
      </div>
      {action}
    </div>
  );
}
