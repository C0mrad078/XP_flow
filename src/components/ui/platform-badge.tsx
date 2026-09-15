import { PlaySquare, Music2, Zap } from "lucide-react";

import { cn } from "@/lib/utilities/cn";
import { PLATFORM_LABELS, type Platform } from "@/types/domain";

const PLATFORM_ICON: Record<Platform, typeof PlaySquare> = {
  youtube: PlaySquare,
  tiktok: Music2,
  kwai: Zap,
};

const PLATFORM_CLASSES: Record<Platform, string> = {
  youtube: "bg-youtube/12 text-youtube border-youtube/25",
  tiktok: "bg-tiktok/12 text-tiktok border-tiktok/25",
  kwai: "bg-kwai/12 text-kwai border-kwai/25",
};

export interface PlatformBadgeProps {
  platform: Platform;
  size?: "sm" | "md";
  iconOnly?: boolean;
  className?: string;
}

export function PlatformBadge({ platform, size = "md", iconOnly = false, className }: PlatformBadgeProps) {
  const Icon = PLATFORM_ICON[platform];

  if (iconOnly) {
    return (
      <span
        className={cn(
          "inline-flex items-center justify-center rounded-md border",
          size === "sm" ? "size-6" : "size-7",
          PLATFORM_CLASSES[platform],
          className,
        )}
        title={PLATFORM_LABELS[platform]}
        aria-label={PLATFORM_LABELS[platform]}
      >
        <Icon className={size === "sm" ? "size-3.5" : "size-4"} />
      </span>
    );
  }

  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium leading-none",
        PLATFORM_CLASSES[platform],
        className,
      )}
    >
      <Icon className="size-3.5" />
      {PLATFORM_LABELS[platform]}
    </span>
  );
}
