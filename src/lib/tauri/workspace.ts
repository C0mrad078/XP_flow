import { invoke } from "./client";
import type { Workspace } from "@/types/domain";

export const workspaceApi = {
  getCurrent: () => invoke<Workspace | null>("get_current_workspace"),
  create: (name: string) => invoke<Workspace>("create_workspace", { name }),
};
