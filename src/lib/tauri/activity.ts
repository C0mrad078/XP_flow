import { invoke } from "./client";
import type { ActivityEvent } from "@/types/domain";

export const activityApi = {
  listRecent: (limit = 100) => invoke<ActivityEvent[]>("list_recent_activity", { limit }),
};
