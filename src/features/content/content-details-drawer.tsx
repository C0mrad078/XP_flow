import type * as React from "react";
import { useState } from "react";
import { AlertTriangle } from "lucide-react";

import { ChannelPicker } from "@/components/common/channel-picker";
import { Badge } from "@/components/ui/badge";
import { Divider } from "@/components/ui/divider";
import { LoadingState } from "@/components/feedback/loading-state";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { Textarea } from "@/components/ui/textarea";
import { useUpdateVideo, useVideoDetail } from "@/hooks/use-content";
import { useSources } from "@/hooks/use-sources";
import { formatBytes } from "@/lib/formatting/number";
import { formatDateTime } from "@/lib/formatting/date";
import { formatBitrate, formatDurationMs, formatFps, formatResolution } from "@/lib/formatting/media";
import { useContentStore } from "@/stores/content-store";
import { VIDEO_PRIORITIES, VIDEO_PRIORITY_LABELS, VIDEO_WARNING_LABELS } from "@/types/media";
import type { VideoDetail } from "@/types/media";

import { VideoStatusBadge } from "./media-status-badge";
import { useVideoActions } from "./use-video-actions";
import { VideoPlayer } from "./video-player";

/** Section 41 — the rich per-video panel. Notes/priority/channel/title
 * autosave on blur/change (optimistic-safe fields per section 72);
 * everything technical is read-only, sourced from the last successful
 * FFprobe run. */
export function ContentDetailsDrawer() {
  const videoId = useContentStore((state) => state.detailVideoId);
  const close = useContentStore((state) => state.closeDetail);
  const { data: detail, isLoading } = useVideoDetail(videoId);

  return (
    <Sheet open={Boolean(videoId)} onOpenChange={(open) => !open && close()}>
      <SheetContent side="right" className="w-full max-w-xl overflow-y-auto">
        <SheetHeader>
          <SheetTitle>Video details</SheetTitle>
        </SheetHeader>

        {isLoading && <LoadingState label="Loading video…" />}
        {/* Keyed by video id so the title/notes draft fields reset cleanly
            when the user opens a different video, without an effect. */}
        {detail && <DetailBody key={detail.id} detail={detail} />}
      </SheetContent>
    </Sheet>
  );
}

function DetailBody({ detail }: { detail: VideoDetail }) {
  const { data: sources = [] } = useSources();
  const updateVideo = useUpdateVideo();
  const actions = useVideoActions(detail);

  const [title, setTitle] = useState(detail.display_title);
  const [notes, setNotes] = useState(detail.notes ?? "");

  return (
    <div className="flex flex-col gap-5 pt-2">
      <div className="overflow-hidden rounded-lg bg-black">
        <VideoPlayer videoId={detail.id} className="aspect-video" />
      </div>

      <div className="flex flex-col gap-1.5">
        <label className="text-label text-foreground">Title</label>
        <input
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          onBlur={() =>
            title.trim() &&
            title !== detail.display_title &&
            updateVideo.mutate({ id: detail.id, input: { display_title: title } })
          }
          className="rounded-md border border-border bg-surface px-3 py-1.5 text-body font-medium text-foreground outline-none focus-visible:ring-2 focus-visible:ring-ring"
        />
        <p className="text-caption normal-case tracking-normal">{detail.original_filename}</p>
      </div>

      <div className="flex flex-wrap items-center gap-2">
        <VideoStatusBadge video={detail} />
        {detail.orientation && <Badge variant="outline">{detail.orientation}</Badge>}
        {detail.aspect_ratio && <Badge variant="outline">{detail.aspect_ratio}</Badge>}
      </div>

      {detail.warnings.length > 0 && (
        <div className="flex flex-col gap-1.5 rounded-md border border-warning/30 bg-warning/[0.06] p-3">
          <div className="flex items-center gap-2 text-warning">
            <AlertTriangle className="size-4" />
            <span className="text-body-small font-medium">Warnings</span>
          </div>
          <ul className="list-inside list-disc text-body-small text-muted-foreground">
            {detail.warnings.map((w) => (
              <li key={w}>{VIDEO_WARNING_LABELS[w]}</li>
            ))}
          </ul>
        </div>
      )}

      {detail.possible_duplicates.length > 0 && (
        <div className="flex flex-col gap-1.5 rounded-md border border-primary/30 bg-primary/[0.05] p-3">
          <p className="text-body-small font-medium text-primary">Possible duplicates</p>
          <p className="text-caption normal-case tracking-normal">
            {detail.possible_duplicates.length} similar video
            {detail.possible_duplicates.length === 1 ? "" : "s"} found — review before publishing both.
          </p>
        </div>
      )}

      <Divider />

      <div className="grid grid-cols-2 gap-x-4 gap-y-3">
        <Field label="Channel">
          <ChannelPicker
            value={detail.channel_id}
            onChange={(channelId) => updateVideo.mutate({ id: detail.id, input: { channel_id: channelId } })}
          />
        </Field>
        <Field label="Priority">
          <Select
            value={detail.priority}
            onValueChange={(value) =>
              updateVideo.mutate({
                id: detail.id,
                input: { priority: value as (typeof VIDEO_PRIORITIES)[number] },
              })
            }
          >
            <SelectTrigger className="h-8">
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
        </Field>
        <Field label="Imported">{formatDateTime(detail.imported_at)}</Field>
        <Field label="File size">{formatBytes(detail.file_size_bytes)}</Field>
        <Field label="Duration">{formatDurationMs(detail.duration_ms)}</Field>
        <Field label="Resolution">{formatResolution(detail.width, detail.height)}</Field>
        <Field label="FPS">{formatFps(detail.fps)}</Field>
        <Field label="Bitrate">{formatBitrate(detail.bitrate)}</Field>
        <Field label="Video codec">{detail.video_codec ?? "-"}</Field>
        <Field label="Audio codec">
          {detail.audio_codec ?? (detail.has_audio === false ? "No audio" : "-")}
        </Field>
        <Field label="Source">{sources.find((s) => s.id === detail.source_id)?.name ?? "-"}</Field>
        <Field label="Hash">
          <span className="font-mono-data text-caption normal-case tracking-normal">
            {detail.content_hash?.slice(0, 16) ?? "-"}
          </span>
        </Field>
      </div>

      <div className="flex flex-col gap-1">
        <p className="text-caption">Path</p>
        <code className="break-all rounded-md border border-border bg-surface-elevated p-2 font-mono-data text-caption normal-case tracking-normal text-foreground">
          {detail.file_path}
        </code>
      </div>

      <div className="flex flex-col gap-1.5">
        <label className="text-label text-foreground">Notes</label>
        <Textarea
          value={notes}
          onChange={(e) => setNotes(e.target.value)}
          onBlur={() =>
            notes !== (detail.notes ?? "") &&
            updateVideo.mutate({ id: detail.id, input: { notes: notes || null } })
          }
          placeholder="Add a note about this video…"
          rows={3}
        />
      </div>

      <Divider />

      <div className="flex flex-wrap gap-2">
        {actions
          .filter((a) => a.key !== "details")
          .map((action) => (
            <button
              key={action.key}
              type="button"
              onClick={action.onSelect}
              className={
                "flex items-center gap-1.5 rounded-md border border-border px-2.5 py-1.5 text-body-small transition-colors hover:bg-surface-hover " +
                (action.destructive ? "text-danger border-danger/30" : "text-foreground")
              }
            >
              <action.icon className="size-3.5" />
              {action.label}
            </button>
          ))}
      </div>

      <p className="text-caption normal-case tracking-normal">
        Publications, metrics and comments for this video will appear here in a future phase.
      </p>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-0.5">
      <p className="text-caption">{label}</p>
      <div className="text-body-small text-foreground">{children}</div>
    </div>
  );
}
