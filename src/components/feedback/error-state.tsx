import { AlertTriangle } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utilities/cn";

export interface ErrorStateProps {
  title?: string;
  description?: string;
  onRetry?: () => void;
  className?: string;
}

export function ErrorState({
  title = "Something went wrong",
  description = "That didn't work. You can try again, or check the Activity log for details.",
  onRetry,
  className,
}: ErrorStateProps) {
  return (
    <div className={cn("flex flex-col items-center justify-center gap-3 px-6 py-16 text-center", className)}>
      <div className="flex size-12 items-center justify-center rounded-xl border border-danger/20 bg-danger/10">
        <AlertTriangle className="size-5 text-danger" />
      </div>
      <div className="flex flex-col gap-1">
        <p className="text-section-title text-foreground">{title}</p>
        <p className="max-w-sm text-body-small text-muted-foreground">{description}</p>
      </div>
      {onRetry && (
        <Button variant="secondary" size="sm" onClick={onRetry}>
          Try again
        </Button>
      )}
    </div>
  );
}
