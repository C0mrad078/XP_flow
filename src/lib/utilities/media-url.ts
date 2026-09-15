/**
 * Local video preview (section 42) and thumbnails are served by a custom
 * Tauri URI scheme (`src-tauri/src/commands/media_protocol.rs`) rather
 * than exposing filesystem paths to the frontend (section 89) — the
 * backend resolves a video id to a path internally on every request.
 */
export function videoStreamUrl(videoId: string): string {
  return `https://xpflowmedia.localhost/video/${videoId}`;
}

export function thumbnailUrl(videoId: string): string {
  return `https://xpflowmedia.localhost/thumbnail/${videoId}`;
}
