import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Film, FolderOpen, FolderTree } from "lucide-react";

import { ChannelPicker } from "@/components/common/channel-picker";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import { useCreateSource } from "@/hooks/use-sources";
import { toast } from "@/stores/toast-store";
import type { VideoSourceType } from "@/types/media";
import { isAppError } from "@/types/domain";
import { cn } from "@/lib/utilities/cn";

type Preset = Extract<VideoSourceType, "cutpro_folder" | "watch_folder">;

/** Section 53/54 — "Add Content Folder": choose a folder, name it, assign
 * a channel, and decide whether XP FLOW watches it going forward. The
 * "Cut.pro Export Folder" preset (section 54) just pre-fills sensible
 * defaults (watch + recursive on) — nothing here is hardcoded to only
 * work with Cut.pro specifically. */
export function AddFolderDialog({
  open: isOpen,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const createSource = useCreateSource();

  const [preset, setPreset] = useState<Preset>("cutpro_folder");
  const [folderPath, setFolderPath] = useState("");
  const [name, setName] = useState("");
  const [channelId, setChannelId] = useState<string | null>(null);
  const [watchEnabled, setWatchEnabled] = useState(true);
  const [recursive, setRecursive] = useState(true);
  const [submitting, setSubmitting] = useState(false);

  function reset() {
    setPreset("cutpro_folder");
    setFolderPath("");
    setName("");
    setChannelId(null);
    setWatchEnabled(true);
    setRecursive(true);
  }

  async function handleChooseFolder() {
    const selection = await open({ directory: true, multiple: false });
    if (!selection || Array.isArray(selection)) return;
    setFolderPath(selection);
    if (!name) {
      const guessedName = selection.split(/[\\/]/).filter(Boolean).pop() ?? "Content Folder";
      setName(guessedName);
    }
  }

  async function handleSubmit() {
    if (!folderPath || !name.trim()) return;
    setSubmitting(true);
    try {
      await createSource.mutateAsync({
        name: name.trim(),
        source_type: preset,
        folder_path: folderPath,
        channel_id: channelId ?? undefined,
        recursive,
        watch_enabled: watchEnabled,
      });
      toast({
        variant: "success",
        title: "Content folder added",
        description: "Indexing started in the background.",
      });
      onOpenChange(false);
      reset();
    } catch (error) {
      toast({
        variant: "error",
        title: "Couldn't add folder",
        description: isAppError(error) ? error.user_message : undefined,
      });
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <Dialog open={isOpen} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Add Content Folder</DialogTitle>
          <DialogDescription>
            XP FLOW will index what's already there and (optionally) watch it for new files.
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-4">
          <div className="grid grid-cols-2 gap-2">
            <PresetButton
              icon={Film}
              label="Cut.pro Export Folder"
              active={preset === "cutpro_folder"}
              onClick={() => {
                setPreset("cutpro_folder");
                setWatchEnabled(true);
              }}
            />
            <PresetButton
              icon={FolderTree}
              label="Custom Watch Folder"
              active={preset === "watch_folder"}
              onClick={() => setPreset("watch_folder")}
            />
          </div>

          <div className="flex flex-col gap-1.5">
            <label className="text-label text-foreground">Folder</label>
            <div className="flex gap-2">
              <Input value={folderPath} readOnly placeholder="Choose a folder…" className="flex-1" />
              <Button type="button" variant="secondary" onClick={handleChooseFolder}>
                <FolderOpen />
                Choose
              </Button>
            </div>
          </div>

          <div className="flex flex-col gap-1.5">
            <label className="text-label text-foreground">Name</label>
            <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="e.g. Football" />
          </div>

          <div className="flex flex-col gap-1.5">
            <label className="text-label text-foreground">Assign to channel</label>
            <ChannelPicker value={channelId} onChange={setChannelId} />
          </div>

          <div className="flex items-center justify-between rounded-md border border-border bg-surface-elevated p-3">
            <div>
              <p className="text-body-small font-medium text-foreground">Watch automatically</p>
              <p className="text-caption normal-case tracking-normal">
                New files are detected and imported as they appear.
              </p>
            </div>
            <Switch checked={watchEnabled} onCheckedChange={setWatchEnabled} />
          </div>

          <div className="flex items-center justify-between rounded-md border border-border bg-surface-elevated p-3">
            <div>
              <p className="text-body-small font-medium text-foreground">Include subfolders</p>
              <p className="text-caption normal-case tracking-normal">Scan nested folders recursively.</p>
            </div>
            <Switch checked={recursive} onCheckedChange={setRecursive} />
          </div>
        </div>

        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={handleSubmit} disabled={!folderPath || !name.trim() || submitting}>
            {submitting ? "Starting…" : "Start indexing"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function PresetButton({
  icon: Icon,
  label,
  active,
  onClick,
}: {
  icon: typeof Film;
  label: string;
  active: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "flex flex-col items-center gap-2 rounded-lg border border-border bg-surface-elevated p-3 text-center transition-colors hover:border-border-strong",
        active && "border-primary bg-primary/5",
      )}
    >
      <Icon className={cn("size-5", active ? "text-primary" : "text-muted-foreground")} />
      <span className="text-body-small font-medium text-foreground">{label}</span>
    </button>
  );
}
