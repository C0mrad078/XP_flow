import { useState } from "react";
import { AlertTriangle, FolderPlus, Pause, Play, RefreshCw, Trash2 } from "lucide-react";

import { AddFolderDialog } from "@/features/content/add-folder-dialog";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { EmptyState } from "@/components/feedback/empty-state";
import { IconButton } from "@/components/ui/icon-button";
import { useChannels } from "@/hooks/use-channels";
import { useDeleteSource, useScanSourceNow, useSources, useUpdateSource } from "@/hooks/use-sources";
import { formatRelativeTime } from "@/lib/formatting/date";
import { formatInteger } from "@/lib/formatting/number";
import { cn } from "@/lib/utilities/cn";
import { confirmAction } from "@/stores/confirm-store";
import { toast } from "@/stores/toast-store";
import { isAppError } from "@/types/domain";

/** Section 52 — Folder Sources, surfaced under Settings → Storage per the
 * brief's explicit allowance ("Create a section accessible through:
 * Settings → Storage or a dedicated Content Sources area"). */
export function SourcesSection() {
  const { data: sources = [], isLoading } = useSources();
  const { data: channels = [] } = useChannels();
  const updateSource = useUpdateSource();
  const deleteSource = useDeleteSource();
  const scanNow = useScanSourceNow();
  const [addOpen, setAddOpen] = useState(false);

  const folderSources = sources.filter((s) => s.source_type !== "manual_import");

  return (
    <Card>
      <CardHeader className="flex-row items-center justify-between space-y-0">
        <div>
          <CardTitle>Content Sources</CardTitle>
          <p className="text-body-small text-muted-foreground">
            Folders XP FLOW watches for new video exports.
          </p>
        </div>
        <Button size="sm" onClick={() => setAddOpen(true)}>
          <FolderPlus />
          Add Folder
        </Button>
      </CardHeader>
      <CardContent>
        {!isLoading && folderSources.length === 0 && (
          <EmptyState
            icon={FolderPlus}
            title="No content folders yet"
            description="Add a Cut.pro export folder or any other watched folder to get started."
          />
        )}

        <div className="flex flex-col divide-y divide-border">
          {folderSources.map((source) => {
            const channelName = channels.find((c) => c.id === source.channel_id)?.name;
            const watching = source.enabled && source.watch_enabled;

            return (
              <div key={source.id} className="flex flex-col gap-2 py-3 first:pt-0 last:pb-0">
                <div className="flex items-start justify-between gap-3">
                  <div className="min-w-0">
                    <div className="flex items-center gap-2">
                      <span
                        className={cn(
                          "size-1.5 rounded-full",
                          source.last_error ? "bg-danger" : watching ? "bg-success" : "bg-muted-foreground",
                        )}
                      />
                      <p className="text-body-small font-medium text-foreground">{source.name}</p>
                      {channelName && (
                        <span className="text-caption normal-case tracking-normal">→ {channelName}</span>
                      )}
                    </div>
                    <p className="truncate font-mono-data text-caption normal-case tracking-normal">
                      {source.folder_path}
                    </p>
                  </div>

                  <div className="flex shrink-0 items-center gap-1">
                    <IconButton
                      label="Scan now"
                      size="sm"
                      onClick={() =>
                        scanNow.mutate(source.id, {
                          onSuccess: (summary) =>
                            toast({
                              variant: "success",
                              title: "Scan complete",
                              description: `${summary.scanned} scanned, ${summary.imported} imported`,
                            }),
                          onError: (error) =>
                            toast({
                              variant: "error",
                              title: "Scan failed",
                              description: isAppError(error) ? error.user_message : undefined,
                            }),
                        })
                      }
                    >
                      <RefreshCw className={cn("size-3.5", scanNow.isPending && "animate-spin")} />
                    </IconButton>
                    <IconButton
                      label={watching ? "Pause watcher" : "Resume watcher"}
                      size="sm"
                      onClick={() =>
                        updateSource.mutate({
                          id: source.id,
                          input: { watch_enabled: !source.watch_enabled },
                        })
                      }
                    >
                      {watching ? <Pause className="size-3.5" /> : <Play className="size-3.5" />}
                    </IconButton>
                    <IconButton
                      label="Remove source"
                      size="sm"
                      onClick={async () => {
                        const confirmed = await confirmAction({
                          title: `Remove "${source.name}"?`,
                          description: "Indexed videos stay in your library.",
                          confirmLabel: "Remove",
                          destructive: true,
                        });
                        if (confirmed) deleteSource.mutate(source.id);
                      }}
                    >
                      <Trash2 className="size-3.5" />
                    </IconButton>
                  </div>
                </div>

                <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-caption normal-case tracking-normal">
                  <span>{source.recursive ? "Includes subfolders" : "Top-level only"}</span>
                  <span>{formatInteger(source.files_indexed)} files indexed</span>
                  <span>
                    {source.last_scan_at
                      ? `Last scan ${formatRelativeTime(source.last_scan_at)}`
                      : "Never scanned"}
                  </span>
                  {source.last_error && (
                    <span className="flex items-center gap-1 text-danger">
                      <AlertTriangle className="size-3" />
                      {source.last_error}
                    </span>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      </CardContent>

      <AddFolderDialog open={addOpen} onOpenChange={setAddOpen} />
    </Card>
  );
}
