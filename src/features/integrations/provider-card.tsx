import { Plus } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { PlatformBadge } from "@/components/ui/platform-badge";
import { Skeleton } from "@/components/ui/skeleton";
import { cn } from "@/lib/utilities/cn";
import type { Channel } from "@/lib/tauri";
import { PLATFORM_LABELS, type Platform } from "@/types/domain";
import type { PlatformAccount } from "@/types/platform-auth";
import type { ProviderConfigurationHealth } from "@/types/provider-configuration-health";

/** Section 61/111 — "integration implemented" is a code-level fact; "provider
 * application approved" is a separate, external fact XP FLOW can't verify or
 * fake. This is a static, honest note per provider, not a live status check. */
const APPROVAL_NOTES: Partial<Record<Platform, string>> = {
  tiktok:
    "Login is implemented, but TikTok's Content Posting API (video.publish) requires a separate app audit before real uploads can go live — tracked for Phase 5.",
  kwai: "Login is implemented against Kwai's Open Platform; publishing scopes require partner approval — tracked for Phase 5.",
};

const STATUS_PILL: Record<ProviderConfigurationHealth["status"], { label: string; className: string }> = {
  ready: { label: "Ready to connect", className: "bg-success/10 text-success" },
  configuration_required: { label: "Configuration required", className: "bg-warning/10 text-warning" },
  broker_unavailable: { label: "Service unavailable", className: "bg-danger/10 text-danger" },
  error: { label: "Unable to check", className: "bg-danger/10 text-danger" },
};

export interface ProviderCardProps {
  platform: Platform;
  accounts: PlatformAccount[];
  channelsById: Map<string, Channel>;
  /** `undefined` while the shared health query is still loading (or
   * failed outright — see `healthQueryFailed`). Never used to decide
   * whether this card renders; it always renders. */
  health: ProviderConfigurationHealth | undefined;
  healthLoading: boolean;
  healthQueryFailed: boolean;
  onConnect: (platform: Platform) => void;
  onManage: (account: PlatformAccount) => void;
  onRetryHealth: () => void;
}

export function ProviderCard({
  platform,
  accounts,
  channelsById,
  health,
  healthLoading,
  healthQueryFailed,
  onConnect,
  onManage,
  onRetryHealth,
}: ProviderCardProps) {
  const note = APPROVAL_NOTES[platform];
  const label = PLATFORM_LABELS[platform];

  return (
    <Card>
      <CardHeader className="flex-row items-center justify-between space-y-0">
        <div className="flex items-center gap-3">
          <PlatformBadge platform={platform} />
          <div>
            <CardTitle>{label}</CardTitle>
            <CardDescription>
              {accounts.length === 0
                ? "No accounts connected"
                : `${accounts.length} account${accounts.length === 1 ? "" : "s"} connected`}
            </CardDescription>
          </div>
        </div>
        <ConnectAction
          platform={platform}
          health={health}
          healthLoading={healthLoading}
          healthQueryFailed={healthQueryFailed}
          onConnect={onConnect}
          onRetryHealth={onRetryHealth}
        />
      </CardHeader>

      <CardContent className="flex flex-col gap-2">
        {healthLoading ? (
          <Skeleton className="h-5 w-40" />
        ) : health ? (
          <span
            className={cn(
              "inline-flex w-fit items-center rounded-full px-2 py-0.5 text-caption normal-case tracking-normal",
              STATUS_PILL[health.status].className,
            )}
          >
            {STATUS_PILL[health.status].label}
          </span>
        ) : null}

        {health?.user_message && (
          <p className="text-caption normal-case tracking-normal text-muted-foreground">
            {health.user_message}
          </p>
        )}

        {accounts.map((account) => (
          <button
            key={account.id}
            type="button"
            onClick={() => onManage(account)}
            className="flex items-center justify-between gap-3 rounded-lg border border-border bg-surface p-3 text-left transition-colors hover:border-border-strong"
          >
            <div className="min-w-0">
              <p className="truncate text-body-small font-medium text-foreground">
                {account.display_name ?? "Unnamed account"}
              </p>
              <p className="text-caption normal-case tracking-normal">
                {channelsById.get(account.channel_id)?.name ?? "Unassigned channel"}
                {account.username_or_handle ? ` · ${account.username_or_handle}` : ""}
              </p>
            </div>
            <span className="shrink-0 text-body-small text-muted-foreground">Manage</span>
          </button>
        ))}

        {note && (
          <p className="mt-1 text-caption normal-case tracking-normal text-muted-foreground">{note}</p>
        )}

        {health && (health.missing_configuration.length > 0 || health.status !== "ready") && (
          <details className="mt-1 text-caption normal-case tracking-normal text-muted-foreground">
            <summary className="cursor-pointer select-none">Diagnostics</summary>
            <dl className="mt-1 grid grid-cols-[auto_1fr] gap-x-2 gap-y-0.5 font-mono text-[11px]">
              <dt>platform</dt>
              <dd>{health.platform}</dd>
              <dt>status</dt>
              <dd>{health.status}</dd>
              <dt>missing</dt>
              <dd>
                {health.missing_configuration.length > 0 ? health.missing_configuration.join(", ") : "—"}
              </dd>
            </dl>
          </details>
        )}
      </CardContent>
    </Card>
  );
}

function ConnectAction({
  platform,
  health,
  healthLoading,
  healthQueryFailed,
  onConnect,
  onRetryHealth,
}: {
  platform: Platform;
  health: ProviderConfigurationHealth | undefined;
  healthLoading: boolean;
  healthQueryFailed: boolean;
  onConnect: (platform: Platform) => void;
  onRetryHealth: () => void;
}) {
  if (healthLoading) {
    return <Skeleton className="h-8 w-32" />;
  }

  if (!health) {
    // The health query itself failed — never leave the card without an
    // action (section 9): offer a retry instead of hiding the button.
    return (
      <Button size="sm" variant="outline" onClick={onRetryHealth} disabled={!healthQueryFailed}>
        Retry
      </Button>
    );
  }

  if (health.status === "broker_unavailable" || health.status === "error") {
    return (
      <Button size="sm" variant="outline" onClick={onRetryHealth}>
        Retry
      </Button>
    );
  }

  if (health.status === "configuration_required") {
    return (
      <Button size="sm" variant="outline" disabled>
        Configure
      </Button>
    );
  }

  return (
    <Button size="sm" variant="outline" onClick={() => onConnect(platform)}>
      <Plus className="size-3.5" />
      {platform === "youtube" ? "Continue with Google" : "Connect account"}
    </Button>
  );
}
