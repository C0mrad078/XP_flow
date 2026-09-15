import type { ActivityEvent } from "@/types/domain";

import { hoursFromNow } from "./time";

/** Demo publication/platform events layered on top of the real,
 * backend-recorded system events (section 30: "The UI may also contain
 * mock publication events for demonstration."). Shaped identically to
 * `ActivityEvent` so they render through the same list/row component as
 * real events — only the `id` prefix marks them as mock. */
export const mockActivityEvents: ActivityEvent[] = [
  {
    id: "mock-1",
    workspace_id: null,
    category: "publication",
    level: "success",
    message: 'Published "5 habits that changed my mornings" to YouTube and TikTok',
    metadata_json: null,
    created_at: hoursFromNow(-3),
  },
  {
    id: "mock-2",
    workspace_id: null,
    category: "publication",
    level: "error",
    message: 'Failed to publish "One-pan garlic butter shrimp" to YouTube — upload timed out',
    metadata_json: null,
    created_at: hoursFromNow(-1.5),
  },
  {
    id: "mock-3",
    workspace_id: null,
    category: "platform",
    level: "warning",
    message: "Clutch Reel: YouTube, TikTok and Kwai sessions need re-authentication",
    metadata_json: null,
    created_at: hoursFromNow(-0.4),
  },
  {
    id: "mock-4",
    workspace_id: null,
    category: "content",
    level: "info",
    message: '"Crispy smashed potatoes, 3 ways" imported from Cut.pro export',
    metadata_json: null,
    created_at: hoursFromNow(-0.25),
  },
  {
    id: "mock-5",
    workspace_id: null,
    category: "warning",
    level: "warning",
    message: "60 Second Kitchen has less than 1 day of queued content remaining",
    metadata_json: null,
    created_at: hoursFromNow(-0.1),
  },
];
