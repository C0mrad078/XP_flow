import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { PublishProgressEvent } from "@/types/publishing";

/** Matches `infrastructure::publishing::tauri_progress_publisher::PUBLISH_PROGRESS_EVENT`. */
const PUBLISH_PROGRESS_EVENT = "publish-progress";

/**
 * The single point that touches `@tauri-apps/api/event` for live upload
 * progress — see `docs/development-guidelines.md`'s "only `src/lib/tauri`
 * imports `@tauri-apps/api` directly" rule. Callers own the returned
 * unlisten function and must call it on cleanup (section 73: no
 * duplicate listeners, no leaks across window lifecycle) — in practice
 * there is exactly one subscriber, mounted once at the app root by
 * `usePublishProgressListener`.
 */
export function subscribeToPublishProgress(
  onEvent: (event: PublishProgressEvent) => void,
): Promise<UnlistenFn> {
  return listen<PublishProgressEvent>(PUBLISH_PROGRESS_EVENT, (event) => onEvent(event.payload));
}
