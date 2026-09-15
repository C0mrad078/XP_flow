import { Play } from "lucide-react";

import { formatDuration } from "@/lib/formatting/number";
import { cn } from "@/lib/utilities/cn";
import { seededGradient } from "@/lib/utilities/gradient";

export interface MediaThumbnailProps {
  seed: string;
  durationSeconds?: number;
  className?: string;
  aspect?: "video" | "portrait";
}

/** Placeholder video thumbnail — a deterministic gradient, not a real
 * frame grab (no transcoding pipeline exists yet, see section 46). Swaps
 * out cleanly once real thumbnail generation lands. */
export function MediaThumbnail({
  seed,
  durationSeconds,
  className,
  aspect = "portrait",
}: MediaThumbnailProps) {
  return (
    <div
      className={cn(
        "relative flex shrink-0 items-center justify-center overflow-hidden rounded-md",
        aspect === "portrait" ? "aspect-9/16" : "aspect-video",
        className,
      )}
      style={{ backgroundImage: seededGradient(seed) }}
    >
      <Play className="size-5 text-white/70" fill="currentColor" />
      {durationSeconds !== undefined && (
        <span className="absolute bottom-1 right-1 rounded bg-black/60 px-1 py-0.5 font-mono-data text-[0.625rem] text-white">
          {formatDuration(durationSeconds)}
        </span>
      )}
    </div>
  );
}
