import { cn } from "@/lib/utilities/cn";
import { AVAILABILITY_STATUS_LABELS, VALIDATION_STATUS_LABELS } from "@/types/media";
import type { AvailabilityStatus, ValidationStatus } from "@/types/media";

type Tone = "neutral" | "info" | "success" | "warning" | "danger";

const TONE_CLASSES: Record<Tone, string> = {
  neutral: "bg-surface-elevated text-muted-foreground border-border",
  info: "bg-primary/10 text-primary border-primary/20",
  success: "bg-success/10 text-success border-success/20",
  warning: "bg-warning/10 text-warning border-warning/20",
  danger: "bg-danger/10 text-danger border-danger/20",
};

const VALIDATION_TONE: Record<ValidationStatus, Tone> = {
  pending: "neutral",
  validating: "info",
  valid: "success",
  invalid: "danger",
  unsupported: "danger",
  corrupted: "danger",
  missing: "warning",
};

const AVAILABILITY_TONE: Record<AvailabilityStatus, Tone> = {
  available: "success",
  missing: "warning",
  moved: "info",
  offline_volume: "warning",
  permission_denied: "danger",
};

function badgeClass(tone: Tone, className?: string) {
  return cn(
    "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium leading-none",
    TONE_CLASSES[tone],
    className,
  );
}

export function ValidationStatusBadge({
  status,
  className,
}: {
  status: ValidationStatus;
  className?: string;
}) {
  return (
    <span className={badgeClass(VALIDATION_TONE[status], className)}>{VALIDATION_STATUS_LABELS[status]}</span>
  );
}

export function AvailabilityStatusBadge({
  status,
  className,
}: {
  status: AvailabilityStatus;
  className?: string;
}) {
  return (
    <span className={badgeClass(AVAILABILITY_TONE[status], className)}>
      {AVAILABILITY_STATUS_LABELS[status]}
    </span>
  );
}

/** Single at-a-glance badge for grid/list rows: availability wins when the
 * file isn't sitting where XP FLOW expects it, otherwise show validation. */
export function VideoStatusBadge({
  video,
  className,
}: {
  video: { validation_status: ValidationStatus; availability_status: AvailabilityStatus };
  className?: string;
}) {
  if (video.availability_status !== "available") {
    return <AvailabilityStatusBadge status={video.availability_status} className={className} />;
  }
  return <ValidationStatusBadge status={video.validation_status} className={className} />;
}
