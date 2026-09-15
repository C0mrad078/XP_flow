import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { PlatformBadge } from "@/components/ui/platform-badge";
import { usePlatformAccountsForWorkspace } from "@/hooks/use-platform-accounts";
import { cn } from "@/lib/utilities/cn";
import { PLATFORMS, PLATFORM_LABELS, type Platform } from "@/types/domain";
import { deriveConnectionHealth } from "@/types/platform-auth";

/** A connected account counts toward "healthy" here if it's usable right
 * now or only mildly at risk — a soon-expiring token isn't a queue-cancelling
 * problem (section 74), only "refresh_required"/"permission_missing"/
 * "provider_error"/"disconnected" are. */
function isOperational(health: ReturnType<typeof deriveConnectionHealth>): boolean {
  return health === "healthy" || health === "token_expiring";
}

/**
 * Section 74's compact per-platform connection-health summary — e.g.
 * "YouTube 2/2 healthy" — distinct from the queue/publication health this
 * dashboard already tracks elsewhere, and from the real-analytics
 * "Platform overview" card (still mock data pending later phases).
 */
export function PlatformHealthSummary() {
  const { data: accounts = [] } = usePlatformAccountsForWorkspace();

  const rows = PLATFORMS.map((platform) => {
    const forPlatform = accounts.filter((a) => a.platform === platform);
    const healthy = forPlatform.filter((a) => isOperational(deriveConnectionHealth(a))).length;
    return { platform, total: forPlatform.length, healthy };
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle>Account health</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        {rows.map(({ platform, total, healthy }) => (
          <PlatformHealthRow key={platform} platform={platform} total={total} healthy={healthy} />
        ))}
      </CardContent>
    </Card>
  );
}

function PlatformHealthRow({
  platform,
  total,
  healthy,
}: {
  platform: Platform;
  total: number;
  healthy: number;
}) {
  const label =
    total === 0
      ? "Not connected"
      : healthy === total
        ? "All healthy"
        : healthy === 0
          ? "Needs attention"
          : "Partial";
  const dotClass =
    total === 0 ? "bg-muted" : healthy === total ? "bg-success" : healthy === 0 ? "bg-danger" : "bg-warning";

  return (
    <div className="flex items-center justify-between gap-3">
      <div className="flex items-center gap-2">
        <PlatformBadge platform={platform} size="sm" iconOnly />
        <span className="text-body-small text-foreground">{PLATFORM_LABELS[platform]}</span>
      </div>
      <div className="flex items-center gap-1.5">
        {total > 0 && (
          <span className="font-mono-data text-body-small text-muted-foreground">
            {healthy}/{total}
          </span>
        )}
        <span className={cn("size-1.5 rounded-full", dotClass)} />
        <span className="text-caption normal-case tracking-normal text-muted-foreground">{label}</span>
      </div>
    </div>
  );
}
