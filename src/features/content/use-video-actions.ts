import type { LucideIcon } from "lucide-react";
import { Archive, ArchiveRestore, Copy, Eye, FolderOpen, Info, RefreshCw, Trash2 } from "lucide-react";

import { contentApi } from "@/lib/tauri";
import {
  useRegenerateThumbnail,
  useRemoveVideo,
  useRevalidateVideo,
  useSetVideoArchived,
} from "@/hooks/use-content";
import { toast } from "@/stores/toast-store";
import { useContentStore } from "@/stores/content-store";
import type { Video } from "@/types/media";
import { isAppError } from "@/types/domain";

export interface VideoAction {
  key: string;
  label: string;
  icon: LucideIcon;
  onSelect: () => void;
  destructive?: boolean;
}

/** Shared action list for the grid card, list row and Content Details
 * panel (section 44: right-click context menu; section 35: grid card
 * hover "More"). One source of truth so both surfaces stay in sync. */
export function useVideoActions(video: Video): VideoAction[] {
  const openDetail = useContentStore((state) => state.openDetail);
  const openQuickPreview = useContentStore((state) => state.openQuickPreview);
  const revalidate = useRevalidateVideo();
  const regenerateThumbnail = useRegenerateThumbnail();
  const setArchived = useSetVideoArchived();
  const remove = useRemoveVideo();

  async function handleReveal() {
    try {
      await contentApi.revealInFileManager(video.id);
    } catch (error) {
      toast({
        variant: "error",
        title: "Couldn't open file location",
        description: isAppError(error) ? error.user_message : undefined,
      });
    }
  }

  async function handleCopyPath() {
    await navigator.clipboard.writeText(video.file_path);
    toast({ variant: "success", title: "File path copied" });
  }

  return [
    { key: "preview", label: "Preview", icon: Eye, onSelect: () => openQuickPreview(video.id) },
    { key: "details", label: "Open Details", icon: Info, onSelect: () => openDetail(video.id) },
    { key: "reveal", label: "Reveal in File Manager", icon: FolderOpen, onSelect: handleReveal },
    { key: "copy-path", label: "Copy File Path", icon: Copy, onSelect: handleCopyPath },
    {
      key: "revalidate",
      label: "Revalidate",
      icon: RefreshCw,
      onSelect: () => {
        revalidate.mutate(video.id, {
          onError: (error) =>
            toast({
              variant: "error",
              title: "Revalidation failed",
              description: isAppError(error) ? error.user_message : undefined,
            }),
        });
      },
    },
    {
      key: "regenerate-thumbnail",
      label: "Regenerate Thumbnail",
      icon: RefreshCw,
      onSelect: () => {
        regenerateThumbnail.mutate(video.id, {
          onError: (error) =>
            toast({
              variant: "error",
              title: "Thumbnail regeneration failed",
              description: isAppError(error) ? error.user_message : undefined,
            }),
        });
      },
    },
    video.archived
      ? {
          key: "unarchive",
          label: "Unarchive",
          icon: ArchiveRestore,
          onSelect: () => setArchived.mutate({ id: video.id, archived: false }),
        }
      : {
          key: "archive",
          label: "Archive",
          icon: Archive,
          onSelect: () => setArchived.mutate({ id: video.id, archived: true }),
        },
    {
      key: "remove",
      label: "Remove from XP FLOW",
      icon: Trash2,
      destructive: true,
      onSelect: () => {
        if (
          !window.confirm(
            `Remove "${video.display_title}" from XP FLOW?\n\nThe original file on disk will NOT be deleted.`,
          )
        ) {
          return;
        }
        remove.mutate(video.id, {
          onError: (error) =>
            toast({
              variant: "error",
              title: "Couldn't remove video",
              description: isAppError(error) ? error.user_message : undefined,
            }),
        });
      },
    },
  ];
}
