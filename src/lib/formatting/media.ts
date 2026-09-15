import { formatDuration } from "./number";

export function formatDurationMs(durationMs: number | null): string {
  if (durationMs === null) return "-";
  return formatDuration(durationMs / 1000);
}

export function formatResolution(width: number | null, height: number | null): string {
  if (width === null || height === null) return "-";
  return width + " x " + height;
}

export function formatFps(fps: number | null): string {
  if (fps === null) return "-";
  return Math.round(fps * 10) / 10 + " fps";
}

export function formatBitrate(bitrate: number | null): string {
  if (bitrate === null || bitrate === 0) return "-";
  const mbps = bitrate / 1_000_000;
  if (mbps >= 1) return mbps.toFixed(1) + " Mbps";
  return Math.round(bitrate / 1000) + " kbps";
}
