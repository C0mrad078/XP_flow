import { useState, type ReactNode } from "react";
import { CheckCircle2, Lock, RefreshCcw, Send, ShieldCheck, Unlock } from "lucide-react";

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
import {
  usePublicationAttempts,
  usePublicationMetadata,
  usePublicationReadiness,
  useProviderRateState,
  usePublishNow,
  useRecordPublicationConsent,
  useRetryPublication,
  useUpdatePublicationMetadata,
} from "@/hooks/use-publishing";
import { usePublicationProgress } from "@/hooks/use-publish-progress";
import {
  isAppError,
  isPublicationOverdue,
  UNKNOWN_REMOTE_RESULT_CODE,
  type Publication,
} from "@/types/domain";
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
  const readiness = usePublicationReadiness(publication.id);
  const rateState = useProviderRateState(publication.platform_account_id);
  const attempts = usePublicationAttempts(publication.id);
  const metadata = usePublicationMetadata(publication.id);
  const publishNow = usePublishNow();
  const retry = useRetryPublication();
  const consent = useRecordPublicationConsent();
  const updateMetadata = useUpdatePublicationMetadata();

  const [draftAt, setDraftAt] = useState("");
  const [metadataDraft, setMetadataDraft] = useState<{
    title: string;
    description: string;
    hashtags: string;
  } | null>(null);
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
  const canPublishNow = ["queued", "scheduled", "failed", "rate_limited", "auth_required", "paused"].includes(
    publication.status,
  );
  const canRetry = publication.status === "failed" || publication.status === "rate_limited";
  const needsConsent = readiness.data?.includes("consent_required");
  // Section 66/67: this is the one case a plain "Retry" must never be
  // offered — the remote outcome is genuinely unknown, and retrying
  // blind is exactly how a duplicate post gets created. Keyed off the
  // stable error code, never status (no PublicationStatus value is ever
  // set specifically for this) or last_error's free text.
  const needsVerification = publication.last_error_code === UNKNOWN_REMOTE_RESULT_CODE;
  const latestAttempt = attempts.data?.[attempts.data.length - 1];
  const liveProgress = usePublicationProgress(publication.id);
  // Live events (section 28) update instantly; the attempt-derived value
  // is the fallback until the first event of this session arrives, and
  // stays accurate even if the event stream was missed for any reason —
  // the persisted attempt is always the authoritative fallback.
  const bytesUploaded = liveProgress?.bytes_uploaded ?? latestAttempt?.bytes_uploaded ?? null;
  const bytesTotal = liveProgress?.bytes_total ?? latestAttempt?.bytes_total ?? null;
  const progress =
    bytesTotal && bytesTotal > 0
      ? Math.min(100, Math.round(((bytesUploaded ?? 0) / bytesTotal) * 100))
      : null;

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

      {readiness.data?.length || needsVerification ? (
        <div className="rounded-md border border-warning/30 bg-warning/[0.06] p-3 text-body-small">
          <p className="font-medium text-warning">
            {needsVerification ? "Needs verification" : "Attention required"}
          </p>
          <p className="mt-1 text-caption normal-case tracking-normal">
            {needsVerification
              ? "XP FLOW could not safely determine whether the provider accepted this publication. Reconcile the remote state before retrying."
              : readiness.data?.map((issue) => issue.split("_").join(" ")).join(" · ")}
          </p>
        </div>
      ) : null}

      {rateState.data?.some((state) => state.limited_until) && (
        <div className="rounded-md border border-warning/30 bg-warning/[0.06] p-3 text-body-small">
          <p className="font-medium text-warning">Provider rate limited</p>
          <p className="mt-1 text-caption normal-case tracking-normal">
            Retry available at{" "}
            {new Date(
              rateState.data.find((state) => state.limited_until)?.limited_until ?? "",
            ).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
            .
          </p>
        </div>
      )}

      <div className="flex flex-wrap gap-2">
        {canPublishNow && !needsVerification && (
          <Button
            size="sm"
            onClick={() => publishNow.mutate(publication.id)}
            disabled={publishNow.isPending || Boolean(readiness.data?.length)}
          >
            <Send className="size-3.5" />
            Publish now
          </Button>
        )}
        {canRetry && !needsVerification && (
          <Button
            size="sm"
            variant="outline"
            onClick={() => retry.mutate(publication.id)}
            disabled={retry.isPending}
          >
            <RefreshCcw className="size-3.5" />
            Retry
          </Button>
        )}
        {needsConsent && (
          <Button
            size="sm"
            variant="outline"
            onClick={() => consent.mutate({ id: publication.id })}
            disabled={consent.isPending}
          >
            <ShieldCheck className="size-3.5" />
            Approve TikTok
          </Button>
        )}
      </div>

      {metadata.data && (
        <div className="rounded-md border border-border bg-surface-elevated p-3">
          <p className="text-caption font-medium uppercase tracking-wider text-muted-foreground">
            Rendered metadata preview
          </p>
          <p className="mt-2 text-body-small font-medium text-foreground">{metadata.data.title}</p>
          <p className="mt-1 line-clamp-3 text-caption normal-case tracking-normal">
            {metadata.data.description}
          </p>
          {metadata.data.hashtags.length > 0 && (
            <p className="mt-2 text-caption normal-case tracking-normal text-primary">
              {metadata.data.hashtags.join(" ")}
            </p>
          )}
          <div className="mt-3 flex flex-col gap-2">
            <input
              aria-label="Publication title override"
              className="h-8 rounded-md border border-border bg-surface px-2 text-body-small text-foreground"
              placeholder="Title override (optional)"
              value={(metadataDraft ?? { title: "", description: "", hashtags: "" }).title}
              onChange={(event) =>
                setMetadataDraft((current) => ({
                  title: event.target.value,
                  description: current?.description ?? "",
                  hashtags: current?.hashtags ?? "",
                }))
              }
            />
            <textarea
              aria-label="Publication description override"
              className="min-h-16 rounded-md border border-border bg-surface px-2 py-1.5 text-body-small text-foreground"
              placeholder="Description override (optional)"
              value={(metadataDraft ?? { title: "", description: "", hashtags: "" }).description}
              onChange={(event) =>
                setMetadataDraft((current) => ({
                  title: current?.title ?? "",
                  description: event.target.value,
                  hashtags: current?.hashtags ?? "",
                }))
              }
            />
            <input
              aria-label="Publication hashtags override"
              className="h-8 rounded-md border border-border bg-surface px-2 text-body-small text-foreground"
              placeholder="#hashtags separated by spaces"
              value={(metadataDraft ?? { title: "", description: "", hashtags: "" }).hashtags}
              onChange={(event) =>
                setMetadataDraft((current) => ({
                  title: current?.title ?? "",
                  description: current?.description ?? "",
                  hashtags: event.target.value,
                }))
              }
            />
            <Button
              size="sm"
              variant="outline"
              disabled={!metadataDraft || updateMetadata.isPending}
              onClick={() =>
                metadataDraft &&
                updateMetadata.mutate({
                  id: publication.id,
                  request: {
                    title_override: { set: true, value: metadataDraft.title || null },
                    description_override: { set: true, value: metadataDraft.description || null },
                    hashtags_override: {
                      set: true,
                      value: metadataDraft.hashtags.split(/\s+/).filter(Boolean),
                    },
                  },
                })
              }
            >
              Save metadata override
            </Button>
          </div>
        </div>
      )}

      {progress !== null && (
        <div className="rounded-md border border-border bg-surface-elevated p-3">
          <div className="flex items-center justify-between text-caption normal-case tracking-normal">
            <span>Upload progress</span>
            <span>{progress}%</span>
          </div>
          <progress
            className="mt-2 h-2 w-full accent-primary"
            max={100}
            value={progress}
            aria-label={`Upload progress ${progress}%`}
          />
          <p className="mt-1 text-caption normal-case tracking-normal text-muted-foreground">
            {(bytesUploaded ?? 0).toLocaleString()} / {(bytesTotal ?? 0).toLocaleString()} bytes
          </p>
        </div>
      )}

      {attempts.data && attempts.data.length > 0 && (
        <div className="flex flex-col gap-2">
          <p className="text-body-small font-medium text-foreground">Attempt history</p>
          {attempts.data.map((attempt) => (
            <div
              key={attempt.id}
              className="flex items-center gap-2 text-caption normal-case tracking-normal"
            >
              <CheckCircle2
                className={
                  attempt.status === "succeeded" ? "size-3.5 text-success" : "size-3.5 text-muted-foreground"
                }
              />
              <span>Attempt {attempt.attempt_number}</span>
              <span className="text-muted-foreground">{attempt.error_message ?? attempt.status}</span>
            </div>
          ))}
        </div>
      )}

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
