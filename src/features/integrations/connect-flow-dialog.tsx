import { CheckCircle2, Loader2, XCircle } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { Platform } from "@/types/domain";
import { PLATFORM_LABELS } from "@/types/domain";
import type { AuthFlowState } from "@/types/platform-auth";

const STATE_LABELS: Record<AuthFlowState["state"], string> = {
  opening_browser: "Opening your browser…",
  waiting_for_authorization: "Waiting for authorization in your browser…",
  verifying_account: "Verifying account…",
  saving_connection: "Saving connection…",
  connected: "Connected",
  failed: "Couldn't connect",
};

export interface ConnectFlowDialogProps {
  platform: Platform;
  state: AuthFlowState | null;
  onCancel: () => void;
  onClose: () => void;
}

/** Section 44's live authorization-progress dialog — the same component
 * backs both the Channels screen's inline "Connect" and the Integrations
 * page, since the underlying flow (`useConnectFlow`) is identical either
 * way. */
export function ConnectFlowDialog({ platform, state, onCancel, onClose }: ConnectFlowDialogProps) {
  if (!state) return null;
  const isTerminal = state.state === "connected" || state.state === "failed";

  return (
    <Dialog open onOpenChange={(open) => !open && (isTerminal ? onClose() : onCancel())}>
      <DialogContent className="max-w-sm" hideClose={!isTerminal}>
        <DialogHeader>
          <DialogTitle>Connecting {PLATFORM_LABELS[platform]}</DialogTitle>
          <DialogDescription>{STATE_LABELS[state.state]}</DialogDescription>
        </DialogHeader>

        <div className="flex flex-col items-center gap-3 py-6">
          {state.state === "connected" && <CheckCircle2 className="size-10 text-success" />}
          {state.state === "failed" && <XCircle className="size-10 text-danger" />}
          {state.state !== "connected" && state.state !== "failed" && (
            <Loader2 className="size-10 animate-spin text-primary" />
          )}
          {state.state === "failed" && (
            <p className="text-center text-body-small text-muted-foreground">{state.message}</p>
          )}
        </div>

        <DialogFooter>
          {isTerminal ? (
            <Button onClick={onClose}>Done</Button>
          ) : (
            <Button variant="outline" onClick={onCancel}>
              Cancel
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
