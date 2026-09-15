import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { useAppInfo } from "@/hooks/use-app-info";

export function AboutSection() {
  const { appInfo } = useAppInfo();

  return (
    <Card>
      <CardHeader>
        <CardTitle>About XP FLOW</CardTitle>
      </CardHeader>
      <CardContent className="grid grid-cols-2 gap-4">
        <InfoRow label="Version" value={appInfo?.version} />
        <InfoRow label="Operating system" value={appInfo ? `${appInfo.os} (${appInfo.arch})` : undefined} />
      </CardContent>
    </Card>
  );
}

function InfoRow({ label, value }: { label: string; value?: string }) {
  return (
    <div>
      <p className="text-caption">{label}</p>
      {value ? (
        <p className="font-mono-data text-body-small text-foreground">{value}</p>
      ) : (
        <Skeleton className="mt-1 h-4 w-24" />
      )}
    </div>
  );
}
