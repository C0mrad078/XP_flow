import { useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

import { useImportFiles } from "@/hooks/use-import";
import { toast } from "@/stores/toast-store";
import { isAppError } from "@/types/domain";
import { SUPPORTED_VIDEO_EXTENSIONS } from "@/types/media";

/** Section 12 — native OS drag-and-drop onto the Content Library. Uses the
 * same `import_files` pipeline as the "Import Videos" dialog (section 12:
 * "Use the same backend ingestion pipeline as all other import methods. Do
 * not create separate logic for drag-and-drop."). Requires
 * `dragDropEnabled: true` in tauri.conf.json so drop events carry real
 * filesystem paths instead of being swallowed as a plain browser event. */
export function useFileDrop() {
  const [isDraggingOver, setIsDraggingOver] = useState(false);
  const importFiles = useImportFiles();

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "over") {
          setIsDraggingOver(true);
          return;
        }
        if (event.payload.type === "leave") {
          setIsDraggingOver(false);
          return;
        }
        if (event.payload.type === "drop") {
          setIsDraggingOver(false);
          const paths = event.payload.paths.filter((path) =>
            SUPPORTED_VIDEO_EXTENSIONS.some((ext) => path.toLowerCase().endsWith(`.${ext}`)),
          );
          if (paths.length === 0) {
            toast({ variant: "warning", title: "No supported video files in that drop" });
            return;
          }
          importFiles.mutate(
            { paths },
            {
              onSuccess: (result) =>
                toast({
                  variant: result.rejected.length > 0 ? "warning" : "success",
                  title: "Import completed",
                  description: `${result.imported} imported${result.rejected.length > 0 ? `, ${result.rejected.length} invalid` : ""}`,
                }),
              onError: (error) =>
                toast({
                  variant: "error",
                  title: "Import failed",
                  description: isAppError(error) ? error.user_message : undefined,
                }),
            },
          );
        }
      })
      .then((fn) => {
        unlisten = fn;
      });

    return () => unlisten?.();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- importFiles is a stable mutation object; re-subscribing on every render would leak listeners.
  }, []);

  return { isDraggingOver };
}
