import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { ChevronDown, FolderInput, Upload } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useImportFiles, useImportFolder } from "@/hooks/use-import";
import { toast } from "@/stores/toast-store";
import { SUPPORTED_VIDEO_EXTENSIONS } from "@/types/media";
import { isAppError } from "@/types/domain";

function summarize(result: {
  imported: number;
  duplicates: number;
  rejected: { path: string; reason: string }[];
}) {
  const parts = [`${result.imported} imported`];
  if (result.duplicates > 0)
    parts.push(`${result.duplicates} duplicate${result.duplicates === 1 ? "" : "s"}`);
  if (result.rejected.length > 0) parts.push(`${result.rejected.length} invalid`);
  return parts.join(", ");
}

/** Section 11 — "Import Videos" covering single/multi-file and whole-folder
 * manual import, all via native OS dialogs (section 11: "Use native
 * operating-system file dialogs through Tauri"). */
export function ImportMenu() {
  const importFiles = useImportFiles();
  const importFolder = useImportFolder();
  const [busy, setBusy] = useState(false);

  async function handleImportFiles() {
    const selection = await open({
      multiple: true,
      filters: [{ name: "Video", extensions: [...SUPPORTED_VIDEO_EXTENSIONS] }],
    });
    if (!selection) return;
    const paths = Array.isArray(selection) ? selection : [selection];

    setBusy(true);
    try {
      const result = await importFiles.mutateAsync({ paths });
      toast({
        variant: result.rejected.length > 0 ? "warning" : "success",
        title: "Import completed",
        description: summarize(result),
      });
    } catch (error) {
      toast({
        variant: "error",
        title: "Import failed",
        description: isAppError(error) ? error.user_message : undefined,
      });
    } finally {
      setBusy(false);
    }
  }

  async function handleImportFolder() {
    const selection = await open({ directory: true, multiple: false });
    if (!selection || Array.isArray(selection)) return;

    setBusy(true);
    try {
      const result = await importFolder.mutateAsync({ folderPath: selection, recursive: true });
      toast({
        variant: result.rejected.length > 0 ? "warning" : "success",
        title: "Folder import completed",
        description: summarize(result),
      });
    } catch (error) {
      toast({
        variant: "error",
        title: "Import failed",
        description: isAppError(error) ? error.user_message : undefined,
      });
    } finally {
      setBusy(false);
    }
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button size="sm" disabled={busy}>
          <Upload />
          {busy ? "Importing…" : "Import"}
          <ChevronDown className="size-3.5" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem onSelect={handleImportFiles}>
          <Upload className="size-3.5" />
          Import videos…
        </DropdownMenuItem>
        <DropdownMenuItem onSelect={handleImportFolder}>
          <FolderInput className="size-3.5" />
          Import folder…
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
