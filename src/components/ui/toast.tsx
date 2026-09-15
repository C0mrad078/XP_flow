import { CheckCircle2, AlertTriangle, XCircle, Info, X } from "lucide-react";

import { cn } from "@/lib/utilities/cn";
import type { ToastItem } from "@/stores/toast-store";

const VARIANT_ICON: Record<ToastItem["variant"], typeof Info> = {
  info: Info,
  success: CheckCircle2,
  warning: AlertTriangle,
  error: XCircle,
};

const VARIANT_CLASSES: Record<ToastItem["variant"], string> = {
  info: "text-primary",
  success: "text-success",
  warning: "text-warning",
  error: "text-danger",
};

export interface ToastProps {
  toast: ToastItem;
  onDismiss: (id: string) => void;
}

export function Toast({ toast: item, onDismiss }: ToastProps) {
  const Icon = VARIANT_ICON[item.variant];
  return (
    <div
      role="status"
      className={cn(
        "pointer-events-auto flex w-80 items-start gap-3 rounded-lg border border-border bg-surface-elevated p-3.5 shadow-xl",
        "animate-in slide-in-from-bottom-2 fade-in-0",
      )}
    >
      <Icon className={cn("mt-0.5 size-4 shrink-0", VARIANT_CLASSES[item.variant])} />
      <div className="min-w-0 flex-1">
        <p className="text-body-small font-medium text-foreground">{item.title}</p>
        {item.description && (
          <p className="text-caption normal-case tracking-normal text-muted-foreground">{item.description}</p>
        )}
      </div>
      <button
        type="button"
        onClick={() => onDismiss(item.id)}
        aria-label="Dismiss notification"
        className="text-muted transition-colors hover:text-foreground"
      >
        <X className="size-3.5" />
      </button>
    </div>
  );
}
