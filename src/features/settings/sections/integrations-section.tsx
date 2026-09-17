import { useMemo, useState } from "react";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { ConnectFlowDialog } from "@/features/integrations/connect-flow-dialog";
import { ManageAccountSheet } from "@/features/integrations/manage-account-sheet";
import { ProviderCard } from "@/features/integrations/provider-card";
import { useChannels } from "@/hooks/use-channels";
import { usePlatformAccountsForWorkspace } from "@/hooks/use-platform-accounts";
import { useConnectFlow, useProviderConfigurationHealth } from "@/hooks/use-platform-auth";
import { useWorkspaceStore } from "@/stores/workspace-store";
import { PLATFORMS, PLATFORM_LABELS, type Platform } from "@/types/domain";
import type { PlatformAccount } from "@/types/platform-auth";
import type { ProviderConfigurationHealth } from "@/types/provider-configuration-health";

/**
 * Section 47-51's real Settings → Integrations screen: one card per
 * provider (YouTube/TikTok/Kwai) listing every connected account across
 * every channel in the workspace, a "Connect account" action that asks
 * which channel to attach the new account to before handing off to the
 * real OAuth flow, and a "Manage" sheet per account (section 50).
 */
export function IntegrationsSection() {
  const workspaceId = useWorkspaceStore((state) => state.workspace?.id);
  const { data: accounts = [] } = usePlatformAccountsForWorkspace();
  const { data: channels = [] } = useChannels();
  const connectFlow = useConnectFlow();
  const {
    data: health,
    isLoading: healthLoading,
    isError: healthQueryFailed,
    refetch: refetchHealth,
  } = useProviderConfigurationHealth();
  const healthByPlatform = useMemo(() => {
    const map = new Map<Platform, ProviderConfigurationHealth>();
    for (const entry of health ?? []) map.set(entry.platform, entry);
    return map;
  }, [health]);

  const [pendingPlatform, setPendingPlatform] = useState<Platform | null>(null);
  const [pendingChannelId, setPendingChannelId] = useState<string | null>(null);
  const [manageAccount, setManageAccount] = useState<PlatformAccount | null>(null);

  const channelsById = useMemo(() => new Map(channels.map((channel) => [channel.id, channel])), [channels]);
  const accountsByPlatform = useMemo(() => {
    const map = new Map<Platform, PlatformAccount[]>();
    for (const platform of PLATFORMS) map.set(platform, []);
    for (const account of accounts) map.get(account.platform)?.push(account);
    return map;
  }, [accounts]);

  function beginConnect() {
    if (!workspaceId || !pendingChannelId || !pendingPlatform) return;
    const platform = pendingPlatform;
    setPendingPlatform(null);
    void connectFlow.connect(workspaceId, pendingChannelId, platform);
  }

  return (
    <div className="flex flex-col gap-4">
      {healthQueryFailed && (
        <div className="flex items-center justify-between gap-3 rounded-lg border border-danger/20 bg-danger/10 p-3 text-body-small text-danger">
          <span>Unable to check provider configuration.</span>
          <Button size="sm" variant="outline" onClick={() => void refetchHealth()}>
            Try Again
          </Button>
        </div>
      )}

      {PLATFORMS.map((platform) => (
        <ProviderCard
          key={platform}
          platform={platform}
          accounts={accountsByPlatform.get(platform) ?? []}
          channelsById={channelsById}
          health={healthByPlatform.get(platform)}
          healthLoading={healthLoading}
          healthQueryFailed={healthQueryFailed}
          onConnect={(p) => {
            setPendingPlatform(p);
            setPendingChannelId(channels[0]?.id ?? null);
          }}
          onManage={setManageAccount}
          onRetryHealth={() => void refetchHealth()}
        />
      ))}

      <Dialog open={pendingPlatform !== null} onOpenChange={(open) => !open && setPendingPlatform(null)}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Connect {pendingPlatform && PLATFORM_LABELS[pendingPlatform]}</DialogTitle>
            <DialogDescription>Which channel should this account be attached to?</DialogDescription>
          </DialogHeader>
          {channels.length === 0 ? (
            <p className="text-body-small text-muted-foreground">
              You don't have any channels yet. Create a channel first, then connect an account to it.
            </p>
          ) : (
            <Select value={pendingChannelId ?? ""} onValueChange={setPendingChannelId}>
              <SelectTrigger className="w-full">
                <SelectValue placeholder="Choose a channel" />
              </SelectTrigger>
              <SelectContent>
                {channels.map((channel) => (
                  <SelectItem key={channel.id} value={channel.id}>
                    {channel.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => setPendingPlatform(null)}>
              Cancel
            </Button>
            <Button onClick={beginConnect} disabled={!pendingChannelId}>
              Continue
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {connectFlow.platform && (
        <ConnectFlowDialog
          platform={connectFlow.platform}
          state={connectFlow.state}
          onCancel={() => void connectFlow.cancel()}
          onClose={() => connectFlow.reset()}
        />
      )}

      <ManageAccountSheet
        account={manageAccount}
        onClose={() => setManageAccount(null)}
        onReconnect={(account) => {
          setManageAccount(null);
          void connectFlow.reconnect(account.id, account.platform);
        }}
      />
    </div>
  );
}
