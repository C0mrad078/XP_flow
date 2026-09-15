import type * as React from "react";
import * as SeparatorPrimitive from "@radix-ui/react-separator";

import { cn } from "@/lib/utilities/cn";

export interface DividerProps extends React.ComponentPropsWithoutRef<typeof SeparatorPrimitive.Root> {
  label?: string;
}

export function Divider({ className, orientation = "horizontal", label, ...props }: DividerProps) {
  if (label && orientation === "horizontal") {
    return (
      <div className="flex items-center gap-3">
        <SeparatorPrimitive.Root
          orientation="horizontal"
          className={cn("h-px flex-1 bg-border", className)}
          {...props}
        />
        <span className="text-caption">{label}</span>
        <SeparatorPrimitive.Root
          orientation="horizontal"
          className={cn("h-px flex-1 bg-border", className)}
        />
      </div>
    );
  }

  return (
    <SeparatorPrimitive.Root
      orientation={orientation}
      className={cn(orientation === "horizontal" ? "h-px w-full" : "h-full w-px", "bg-border", className)}
      {...props}
    />
  );
}
