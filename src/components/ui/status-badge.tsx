import { cn } from "@/lib/utilities/cn";
import type { PublicationStatus } from "@/types/domain";

type StatusTone = "neutral" | "info" | "success" | "warning" | "danger" | "active";

const STATUS_TONE: Record<PublicationStatus, StatusTone> = {
  imported: "neutral",
  validating: "info",
  ready: "neutral",
  queued: "info",
  scheduled: "info",
  uploading: "active",
  processing: "active",
  published: "success",
  failed: "danger",
  retry_wait: "warning",
  auth_required: "warning",
  rate_limited: "warning",
  blocked: "danger",
  paused: "neutral",
  cancelled: "neutral",
  archived: "neutral",
  duplicate: "warning",
};

const STATUS_LABEL: Record<PublicationStatus, string> = {
  imported: "Imported",
  validating: "Validating",
  ready: "Ready",
  queued: "Queued",
  scheduled: "Scheduled",
  uploading: "Uploading",
  processing: "Processing",
  published: "Published",
  failed: "Failed",
  retry_wait: "Retrying",
  auth_required: "Auth required",
  rate_limited: "Rate limited",
  blocked: "Blocked",
  paused: "Paused",
  cancelled: "Cancelled",
  archived: "Archived",
  duplicate: "Duplicate",
};

const TONE_CLASSES: Record<StatusTone, string> = {
  neutral: "bg-surface-elevated text-muted-foreground border-border",
  info: "bg-primary/10 text-primary border-primary/20",
  success: "bg-success/10 text-success border-success/20",
  warning: "bg-warning/10 text-warning border-warning/20",
  danger: "bg-danger/10 text-danger border-danger/20",
  active: "bg-primary/10 text-primary border-primary/20",
};

const DOT_CLASSES: Record<StatusTone, string> = {
  neutral: "bg-muted-foreground",
  info: "bg-primary",
  success: "bg-success",
  warning: "bg-warning",
  danger: "bg-danger",
  active: "bg-primary",
};

export interface StatusBadgeProps {
  status: PublicationStatus;
  className?: string;
}

export function StatusBadge({ status, className }: StatusBadgeProps) {
  const tone = STATUS_TONE[status];
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium leading-none",
        TONE_CLASSES[tone],
        className,
      )}
    >
      <span
        className={cn("size-1.5 rounded-full", DOT_CLASSES[tone], tone === "active" && "animate-pulse")}
        aria-hidden
      />
      {STATUS_LABEL[status]}
    </span>
  );
}
