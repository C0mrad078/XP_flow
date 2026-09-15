import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useAppInfo } from "@/hooks/use-app-info";

export function StorageSection() {
  const { appInfo } = useAppInfo();

  return (
    <Card>
      <CardHeader>
        <CardTitle>Storage locations</CardTitle>
        <CardDescription>
          XP FLOW keeps everything local to this machine (section 2.1 — local-first).
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <StorageRow label="Database" value={appInfo?.database_path} />
        <StorageRow label="Application data" value={appInfo?.data_dir} />
        <StorageRow label="Logs" value={appInfo?.log_dir} />
      </CardContent>
    </Card>
  );
}

function StorageRow({ label, value }: { label: string; value?: string }) {
  return (
    <div className="flex flex-col gap-1 rounded-md border border-border bg-surface-elevated p-3">
      <span className="text-caption">{label}</span>
      {value ? (
        <code className="break-all font-mono-data text-body-small text-foreground">{value}</code>
      ) : (
        <Skeleton className="h-4 w-full max-w-sm" />
      )}
    </div>
  );
}
