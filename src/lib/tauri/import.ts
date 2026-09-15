import { invoke } from "./client";
import type { ImportSummary } from "@/types/media";
import type { UUID } from "@/types/domain";

export const importApi = {
  files: (workspaceId: UUID, paths: string[], channelId?: UUID) =>
    invoke<ImportSummary>("import_files", { workspaceId, paths, channelId }),
  folder: (workspaceId: UUID, folderPath: string, recursive: boolean, channelId?: UUID) =>
    invoke<ImportSummary>("import_folder", { workspaceId, folderPath, recursive, channelId }),
};
