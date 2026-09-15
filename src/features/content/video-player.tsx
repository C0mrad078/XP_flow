import { videoStreamUrl } from "@/lib/utilities/media-url";
import { cn } from "@/lib/utilities/cn";

export interface VideoPlayerProps {
  videoId: string;
  className?: string;
  autoPlay?: boolean;
}

/** Local preview (section 42) — plain HTML5 `<video>` against the
 * `xpflowmedia://` protocol, which streams by video id with HTTP Range
 * support (see `src-tauri/src/commands/media_protocol.rs`) so seeking
 * never loads a whole file into memory. Native controls give play/pause/
 * seek/volume/time for free without extra dependencies. */
export function VideoPlayer({ videoId, className, autoPlay = false }: VideoPlayerProps) {
  return (
    <video
      key={videoId}
      src={videoStreamUrl(videoId)}
      controls
      autoPlay={autoPlay}
      playsInline
      className={cn("h-full w-full bg-black", className)}
    />
  );
}
