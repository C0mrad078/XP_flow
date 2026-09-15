import { useEffect, useState } from "react";
import { CheckCircle2, XCircle } from "lucide-react";

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { systemApi } from "@/lib/tauri";
import { cn } from "@/lib/utilities/cn";
import type { MediaToolchainStatus, MediaToolStatus } from "@/types/domain";

export function AdvancedSection() {
  const [status, setStatus] = useState<MediaToolchainStatus | null>(null);

  useEffect(() => {
    systemApi
      .getMediaStatus()
      .then(setStatus)
      .catch(() => setStatus(null));
  }, []);

  return (
    <Card>
      <CardHeader>
        <CardTitle>Media toolchain</CardTitle>
        <CardDescription>
          FFmpeg/FFprobe detection (section 46) — transcoding itself isn't implemented yet, this just reports
          whether the tools are available on this machine.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        {status ? (
          <>
            <ToolRow name="FFmpeg" tool={status.ffmpeg} />
            <ToolRow name="FFprobe" tool={status.ffprobe} />
          </>
        ) : (
          <>
            <Skeleton className="h-12 w-full" />
            <Skeleton className="h-12 w-full" />
          </>
        )}
      </CardContent>
    </Card>
  );
}

function ToolRow({ name, tool }: { name: string; tool: MediaToolStatus }) {
  return (
    <div className="flex items-center justify-between rounded-md border border-border bg-surface-elevated p-3">
      <div className="flex items-center gap-2.5">
        {tool.available ? (
          <CheckCircle2 className="size-4 text-success" />
        ) : (
          <XCircle className="size-4 text-muted" />
        )}
        <div>
          <p className="text-body-small font-medium text-foreground">{name}</p>
          {tool.path && (
            <p className="font-mono-data text-caption normal-case tracking-normal">{tool.path}</p>
          )}
        </div>
      </div>
      <span className={cn("text-body-small", tool.available ? "text-success" : "text-muted-foreground")}>
        {tool.available ? "Available" : "Not found"}
      </span>
    </div>
  );
}
