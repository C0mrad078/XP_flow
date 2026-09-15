import type * as React from "react";

import { cn } from "@/lib/utilities/cn";

export function Skeleton({ className, ...props }: React.HTMLAttributes<HTMLDivElement>) {
  return <div className={cn("animate-pulse rounded-md bg-surface-hover", className)} {...props} />;
}
