import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { SourcesSection } from "./sources-section";
import { useAppInfo } from "@/hooks/use-app-info";
import { systemApi } from "@/lib/tauri";
import { formatBytes } from "@/lib/formatting/number";
import { toast } from "@/stores/toast-store";
import { isAppError } from "@/types/domain";

export function StorageSection() {
  const { appInfo } = useAppInfo();
  const queryClient = useQueryClient();
  const { data: cacheInfo } = useQuery({
    queryKey: ["cache-info"],
    queryFn: systemApi.getCacheInfo,
    refetchInterval: 30_000,
  });
  const [clearing, setClearing] = useState(false);

  async function handleClearTempCache() {
    setClearing(true);
    try {
      await systemApi.clearTempCache();
      await queryClient.invalidateQueries({ queryKey: ["cache-info"] });
      toast({ variant: "success", title: "Temporary cache cleared" });
    } catch (error) {
      toast({
        variant: "error",
        title: "Couldn't clear cache",
        description: isAppError(error) ? error.user_message : undefined,
      });
    } finally {
      setClearing(false);
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <Card>
        <CardHeader>
          <CardTitle>Storage locations</CardTitle>
          <CardDescription>
            XP FLOW keeps everything local to this machine (section 2.1 — local-first).
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <StorageRow label="Database" value={appInfo?.database_path} size={cacheInfo?.database_size_bytes} />
          <StorageRow label="Application data" value={appInfo?.data_dir} />
          <StorageRow label="Logs" value={appInfo?.log_dir} />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Cache</CardTitle>
          <CardDescription>
            Generated thumbnails and scratch space (section 66/67) — never stored beside your original videos.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <div className="grid grid-cols-2 gap-3">
            <CacheTile label="Thumbnail cache" bytes={cacheInfo?.thumbnail_cache_size_bytes} />
            <CacheTile label="Temporary cache" bytes={cacheInfo?.temp_cache_size_bytes} />
          </div>
          <Button
            variant="outline"
            size="sm"
            className="w-fit"
            onClick={handleClearTempCache}
            disabled={clearing}
          >
            <Trash2 />
            {clearing ? "Clearing…" : "Clear temporary cache"}
          </Button>
          <p className="text-caption normal-case tracking-normal">
            This only clears scratch files — thumbnails are kept.
          </p>
        </CardContent>
      </Card>

      <SourcesSection />
    </div>
  );
}

function StorageRow({ label, value, size }: { label: string; value?: string; size?: number }) {
  return (
    <div className="flex flex-col gap-1 rounded-md border border-border bg-surface-elevated p-3">
      <div className="flex items-center justify-between">
        <span className="text-caption">{label}</span>
        {size !== undefined && (
          <span className="font-mono-data text-caption normal-case tracking-normal">{formatBytes(size)}</span>
        )}
      </div>
      {value ? (
        <code className="break-all font-mono-data text-body-small text-foreground">{value}</code>
      ) : (
        <Skeleton className="h-4 w-full max-w-sm" />
      )}
    </div>
  );
}

function CacheTile({ label, bytes }: { label: string; bytes?: number }) {
  return (
    <div className="rounded-md border border-border bg-surface-elevated p-3">
      <p className="text-caption">{label}</p>
      {bytes === undefined ? (
        <Skeleton className="mt-1 h-5 w-16" />
      ) : (
        <p className="text-metric-small text-foreground">{formatBytes(bytes)}</p>
      )}
    </div>
  );
}
