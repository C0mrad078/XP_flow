import { UploadCloud } from "lucide-react";

export function DropZoneOverlay({ visible }: { visible: boolean }) {
  if (!visible) return null;

  return (
    <div className="pointer-events-none fixed inset-0 z-(--z-overlay) flex items-center justify-center bg-background/80 backdrop-blur-sm">
      <div className="flex flex-col items-center gap-3 rounded-xl border-2 border-dashed border-primary bg-surface-elevated px-12 py-10">
        <UploadCloud className="size-8 text-primary" />
        <p className="text-section-title text-foreground">Drop videos to import</p>
        <p className="text-body-small text-muted-foreground">MP4, MOV, MKV, WEBM</p>
      </div>
    </div>
  );
}
