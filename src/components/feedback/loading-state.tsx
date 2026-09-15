import { Loader2 } from "lucide-react";

import { cn } from "@/lib/utilities/cn";

export interface LoadingStateProps {
  label?: string;
  className?: string;
}

export function LoadingState({ label = "Loading…", className }: LoadingStateProps) {
  return (
    <div className={cn("flex flex-col items-center justify-center gap-2 px-6 py-16 text-center", className)}>
      <Loader2 className="size-5 animate-spin text-muted-foreground" />
      <p className="text-body-small text-muted-foreground">{label}</p>
    </div>
  );
}
