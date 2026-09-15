import { invoke } from "./client";
import type {
  CreateSourceInput,
  ReconcileSummary,
  SourceSummary,
  UpdateSourceInput,
  VideoSource,
} from "@/types/media";
import type { UUID } from "@/types/domain";

export const sourcesApi = {
  list: (workspaceId: UUID) => invoke<SourceSummary[]>("list_sources", { workspaceId }),
  create: (workspaceId: UUID, input: CreateSourceInput) =>
    invoke<VideoSource>("create_source", { workspaceId, input }),
  update: (id: UUID, input: UpdateSourceInput) => invoke<VideoSource>("update_source", { id, input }),
  remove: (id: UUID) => invoke<void>("delete_source", { id }),
  scanNow: (id: UUID) => invoke<ReconcileSummary>("scan_source_now", { id }),
};
