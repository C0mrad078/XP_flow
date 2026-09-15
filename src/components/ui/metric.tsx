import type { LucideIcon } from "lucide-react";
import { ArrowDownRight, ArrowUpRight } from "lucide-react";

import { cn } from "@/lib/utilities/cn";

export interface MetricProps {
  label: string;
  value: string;
  icon?: LucideIcon;
  trend?: {
    direction: "up" | "down";
    value: string;
    /** Whether "up" is a good outcome for this metric (defaults to true). */
    positiveIsGood?: boolean;
  };
  className?: string;
}

export function Metric({ label, value, icon: Icon, trend, className }: MetricProps) {
  const trendIsGood = trend && (trend.positiveIsGood ?? true ? trend.direction === "up" : trend.direction === "down");

  return (
    <div className={cn("flex flex-col gap-2", className)}>
      <div className="flex items-center gap-2 text-muted-foreground">
        {Icon && <Icon className="size-3.5" />}
        <span className="text-caption">{label}</span>
      </div>
      <div className="flex items-baseline gap-2">
        <span className="text-metric-large text-foreground">{value}</span>
        {trend && (
          <span
            className={cn(
              "inline-flex items-center gap-0.5 text-xs font-medium",
              trendIsGood ? "text-success" : "text-danger",
            )}
          >
            {trend.direction === "up" ? <ArrowUpRight className="size-3.5" /> : <ArrowDownRight className="size-3.5" />}
            {trend.value}
          </span>
        )}
      </div>
    </div>
  );
}
