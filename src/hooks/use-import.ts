import { useMutation, useQueryClient } from "@tanstack/react-query";

import { importApi } from "@/lib/tauri";
import { useWorkspaceStore } from "@/stores/workspace-store";

function useInvalidateAfterImport() {
  const queryClient = useQueryClient();
  return () => {
    queryClient.invalidateQueries({ queryKey: ["content"] });
    queryClient.invalidateQueries({ queryKey: ["content-summary"] });
  };
}

export function useImportFiles() {
  const invalidate = useInvalidateAfterImport();
  const workspaceId = useWorkspaceStore((state) => state.workspace?.id);
  return useMutation({
    mutationFn: ({ paths, channelId }: { paths: string[]; channelId?: string }) =>
      importApi.files(workspaceId!, paths, channelId),
    onSuccess: invalidate,
  });
}

export function useImportFolder() {
  const invalidate = useInvalidateAfterImport();
  const workspaceId = useWorkspaceStore((state) => state.workspace?.id);
  return useMutation({
    mutationFn: ({
      folderPath,
      recursive,
      channelId,
    }: {
      folderPath: string;
      recursive: boolean;
      channelId?: string;
    }) => importApi.folder(workspaceId!, folderPath, recursive, channelId),
    onSuccess: invalidate,
  });
}
