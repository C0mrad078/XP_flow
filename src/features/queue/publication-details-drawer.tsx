import { useState, type ReactNode } from "react";
import { Lock, Unlock } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Divider } from "@/components/ui/divider";
import { LoadingState } from "@/components/feedback/loading-state";
import { PlatformBadge } from "@/components/ui/platform-badge";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { StatusBadge } from "@/components/ui/status-badge";
import { useChannels } from "@/hooks/use-channels";
import {
  useAutoSchedulePublication,
  useSchedulePublication,
  useUnschedulePublication,
} from "@/hooks/use-scheduler";
import {
  useArchivePublication,
  useCancelPublication,
  usePublication,
  useSetPublicationLocked,
  useSetPublicationPriority,
} from "@/hooks/use-queue";
import { formatDateTimeInZone } from "@/lib/formatting/date";
import { confirmAction } from "@/stores/confirm-store";
import { toast } from "@/stores/toast-store";
import { useWorkspaceStore } from "@/stores/workspace-store";
import { isAppError, isPublicationOverdue, type Publication } from "@/types/domain";
import { VIDEO_PRIORITIES, VIDEO_PRIORITY_LABELS, type VideoPriority } from "@/types/media";

export interface PublicationDetailsDrawerProps {
  publicationId: string | null;
  onClose: () => void;
}

export function PublicationDetailsDrawer({ publicationId, onClose }: PublicationDetailsDrawerProps) {
  const { data: publication, isLoading } = usePublication(publicationId);

  return (
    <Sheet open={Boolean(publicationId)} onOpenChange={(open) => !open && onClose()}>
      <SheetContent side="right" className="w-full max-w-md overflow-y-auto">
        <SheetHeader>
          <SheetTitle>Publication details</SheetTitle>
        </SheetHeader>

        {isLoading && <LoadingState label="Loading publication…" />}
        {publication && <DetailBody key={publication.id} publication={publication} onClose={onClose} />}
      </SheetContent>
    </Sheet>
  );
}

function DetailBody({ publication, onClose }: { publication: Publication; onClose: () => void }) {
  const { data: channels = [] } = useChannels();
  const channel = channels.find((c) => c.id === publication.channel_id);
  const timezone = useWorkspaceStore((state) => state.workspace?.timezone ?? "UTC");

  const setPriority = useSetPublicationPriority();
  const setLocked = useSetPublicationLocked();
  const cancel = useCancelPublication();
  const archive = useArchivePublication();
  const scheduleAt = useSchedulePublication();
  const autoSchedule = useAutoSchedulePublication();
  const unschedule = useUnschedulePublication();

  const [draftAt, setDraftAt] = useState("");
  const overdue = isPublicationOverdue(publication);

  function toDatetimeLocalValue(iso: string | null): string {
    if (!iso) return "";
    const date = new Date(iso);
    const pad = (n: number) => String(n).padStart(2, "0");
    return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}`;
  }

  function handleScheduleSubmit() {
    const value = draftAt || toDatetimeLocalValue(publication.scheduled_at);
    if (!value) return;
    scheduleAt.mutate(
      { publicationId: publication.id, at: new Date(value).toISOString() },
      {
        onError: (error) =>
          toast({
            variant: "error",
            title: "Couldn't schedule",
            description: isAppError(error) ? error.user_message : undefined,
          }),
      },
    );
  }

  async function handleCancel() {
    const confirmed = await confirmAction({
      title: "Cancel this publication?",
      description: "It will be removed from the queue. This can't be undone from here.",
      confirmLabel: "Cancel Publication",
      destructive: true,
    });
    if (!confirmed) return;
    cancel.mutate(publication.id, {
      onSuccess: onClose,
      onError: (error) =>
        toast({
          variant: "error",
          title: "Couldn't cancel",
          description: isAppError(error) ? error.user_message : undefined,
        }),
    });
  }

  const canSchedule = publication.status === "queued" || publication.status === "scheduled";
  const canCancel = publication.status !== "cancelled" && publication.status !== "archived";

  return (
    <div className="flex flex-col gap-5">
      <div>
        <p className="text-body font-medium text-foreground">{publication.title}</p>
        <div className="mt-1.5 flex flex-wrap items-center gap-1.5">
          <PlatformBadge platform={publication.platform} size="sm" />
          <StatusBadge status={publication.status} />
          {overdue && <Badge variant="danger">Overdue</Badge>}
          {publication.locked && <Badge variant="outline">Locked</Badge>}
        </div>
      </div>

      <Divider />

      <div className="flex flex-col gap-3 text-body-small">
        <Row label="Channel">{channel?.name ?? "Unknown"}</Row>
        <Row label="Priority">
          <Select
            value={publication.priority}
            onValueChange={(value) =>
              setPriority.mutate({ id: publication.id, priority: value as VideoPriority })
            }
          >
            <SelectTrigger className="h-8 w-36">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {VIDEO_PRIORITIES.map((p) => (
                <SelectItem key={p} value={p}>
                  {VIDEO_PRIORITY_LABELS[p]}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </Row>
        <Row label="Scheduled">
          {publication.scheduled_at
            ? formatDateTimeInZone(publication.scheduled_at, timezone)
            : "Not scheduled"}
        </Row>
        <Row label="Locked">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setLocked.mutate({ id: publication.id, locked: !publication.locked })}
          >
            {publication.locked ? <Lock className="size-3.5" /> : <Unlock className="size-3.5" />}
            {publication.locked ? "Unlock" : "Lock"}
          </Button>
        </Row>
      </div>

      {canSchedule && (
        <>
          <Divider />
          <div className="flex flex-col gap-2">
            <span className="text-body-small font-medium text-foreground">Schedule</span>
            <input
              type="datetime-local"
              className="h-9 rounded-md border border-border bg-surface px-3 text-body-small text-foreground"
              defaultValue={toDatetimeLocalValue(publication.scheduled_at)}
              onChange={(e) => setDraftAt(e.target.value)}
            />
            <div className="flex flex-wrap gap-2">
              <Button size="sm" onClick={handleScheduleSubmit} disabled={scheduleAt.isPending}>
                {publication.scheduled_at ? "Reschedule" : "Schedule"}
              </Button>
              {publication.status === "queued" && (
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() =>
                    autoSchedule.mutate(publication.id, {
                      onError: (error) =>
                        toast({
                          variant: "error",
                          title: "Couldn't auto-schedule",
                          description: isAppError(error) ? error.user_message : undefined,
                        }),
                    })
                  }
                  disabled={autoSchedule.isPending}
                >
                  Auto-schedule
                </Button>
              )}
              {publication.status === "scheduled" && (
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => unschedule.mutate(publication.id)}
                  disabled={unschedule.isPending}
                >
                  Unschedule
                </Button>
              )}
            </div>
          </div>
        </>
      )}

      <Divider />

      <div className="flex flex-wrap gap-2">
        {canCancel && (
          <Button variant="destructive" size="sm" onClick={handleCancel} disabled={cancel.isPending}>
            Cancel Publication
          </Button>
        )}
        {publication.status === "cancelled" && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => archive.mutate(publication.id)}
            disabled={archive.isPending}
          >
            Archive
          </Button>
        )}
      </div>
    </div>
  );
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-3">
      <span className="text-muted-foreground">{label}</span>
      <span className="text-foreground">{children}</span>
    </div>
  );
}
