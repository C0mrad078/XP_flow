import { Plus } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { PlatformBadge } from "@/components/ui/platform-badge";
import type { Channel } from "@/lib/tauri";
import { PLATFORM_LABELS, type Platform } from "@/types/domain";
import type { PlatformAccount } from "@/types/platform-auth";

/** Section 61/111 — "integration implemented" is a code-level fact; "provider
 * application approved" is a separate, external fact XP FLOW can't verify or
 * fake. This is a static, honest note per provider, not a live status check. */
const APPROVAL_NOTES: Partial<Record<Platform, string>> = {
  tiktok:
    "Login is implemented, but TikTok's Content Posting API (video.publish) requires a separate app audit before real uploads can go live — tracked for Phase 5.",
  kwai: "Login is implemented against Kwai's Open Platform; publishing scopes require partner approval — tracked for Phase 5.",
};

export interface ProviderCardProps {
  platform: Platform;
  accounts: PlatformAccount[];
  channelsById: Map<string, Channel>;
  onConnect: (platform: Platform) => void;
  onManage: (account: PlatformAccount) => void;
}

export function ProviderCard({ platform, accounts, channelsById, onConnect, onManage }: ProviderCardProps) {
  const note = APPROVAL_NOTES[platform];

  return (
    <Card>
      <CardHeader className="flex-row items-center justify-between space-y-0">
        <div className="flex items-center gap-3">
          <PlatformBadge platform={platform} />
          <div>
            <CardTitle>{PLATFORM_LABELS[platform]}</CardTitle>
            <CardDescription>
              {accounts.length === 0
                ? "No accounts connected"
                : `${accounts.length} account${accounts.length === 1 ? "" : "s"} connected`}
            </CardDescription>
          </div>
        </div>
        <Button size="sm" variant="outline" onClick={() => onConnect(platform)}>
          <Plus className="size-3.5" />
          Connect account
        </Button>
      </CardHeader>

      <CardContent className="flex flex-col gap-2">
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
      </CardContent>
    </Card>
  );
}
