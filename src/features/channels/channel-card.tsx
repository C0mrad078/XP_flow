import { CalendarClock, Pause, Play, Plus } from "lucide-react";

import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { useSetChannelStatus } from "@/hooks/use-channels";
import { useConnectFlow } from "@/hooks/use-platform-auth";
import { cn } from "@/lib/utilities/cn";
import type { ChannelOverview } from "@/lib/tauri";
import { useWorkspaceStore } from "@/stores/workspace-store";
import { PLATFORMS, PLATFORM_LABELS, type Platform } from "@/types/domain";

import { ConnectFlowDialog } from "@/features/integrations/connect-flow-dialog";
import { PlatformAccountRow } from "@/features/integrations/platform-account-row";

export interface ChannelCardProps {
  overview: ChannelOverview;
  onOpenSchedule: (channelId: string) => void;
}

export function ChannelCard({ overview, onOpenSchedule }: ChannelCardProps) {
  const {
    channel,
    platform_accounts: accounts,
    queued_count: queuedCount,
    active_slot_count: activeSlotsPerWeek,
  } = overview;
  const workspaceId = useWorkspaceStore((state) => state.workspace?.id);
  const setStatus = useSetChannelStatus();
  const connectFlow = useConnectFlow();
  const contentStockDays =
    activeSlotsPerWeek > 0 ? Math.floor(queuedCount / (activeSlotsPerWeek / 7)) : queuedCount > 0 ? null : 0;
  const isPaused = channel.status === "paused";
  const connectingPlatform = connectFlow.state ? connectFlow.platform : null;

  function handleConnect(platform: Platform) {
    if (!workspaceId) return;
    void connectFlow.connect(workspaceId, channel.id, platform);
  }

  return (
    <Card>
      <CardHeader className="flex-row items-start justify-between space-y-0">
        <div className="flex items-center gap-3">
          <Avatar className="size-10">
            <AvatarFallback className="text-sm">{channel.name.slice(0, 2).toUpperCase()}</AvatarFallback>
          </Avatar>
          <div>
            <p className="text-section-title text-foreground">{channel.name}</p>
            <p className="text-body-small text-muted-foreground">{channel.niche ?? "No niche set"}</p>
          </div>
        </div>
        <div className="flex items-center gap-1">
          <Button variant="ghost" size="sm" onClick={() => onOpenSchedule(channel.id)}>
            <CalendarClock className="size-3.5" />
            Schedule
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setStatus.mutate({ id: channel.id, status: isPaused ? "active" : "paused" })}
            disabled={setStatus.isPending}
          >
            {isPaused ? <Play className="size-3.5" /> : <Pause className="size-3.5" />}
            {isPaused ? "Paused" : "Active"}
          </Button>
        </div>
      </CardHeader>

      <CardContent className="flex flex-col gap-4">
        <div className="flex flex-col gap-2">
          {accounts.map((account) => (
            <PlatformAccountRow
              key={account.id}
              account={account}
              onReconnect={() => connectFlow.reconnect(account.id, account.platform)}
            />
          ))}
          <Select value="" onValueChange={(value) => handleConnect(value as Platform)}>
            <SelectTrigger className="h-7 w-fit gap-1.5 border-dashed px-2 text-caption normal-case tracking-normal [&>svg:last-child]:hidden">
              <Plus className="size-3.5" />
              <SelectValue placeholder="Connect account" />
            </SelectTrigger>
            <SelectContent>
              {PLATFORMS.map((p) => (
                <SelectItem key={p} value={p}>
                  {PLATFORM_LABELS[p]}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="grid grid-cols-2 gap-3 border-t border-border pt-3">
          <div>
            <p className="text-metric-small text-foreground">{queuedCount}</p>
            <p className="text-caption">Queued videos</p>
          </div>
          <div>
            <p
              className={cn(
                "text-metric-small",
                contentStockDays !== null && contentStockDays <= 2 ? "text-danger" : "text-foreground",
              )}
            >
              {contentStockDays === null ? "—" : `${contentStockDays}d`}
            </p>
            <p className="text-caption">Content stock</p>
          </div>
        </div>

        {contentStockDays !== null && contentStockDays <= 2 && (
          <Badge variant="danger" className="w-fit">
            Running low on content
          </Badge>
        )}
      </CardContent>

      {connectingPlatform && (
        <ConnectFlowDialog
          platform={connectingPlatform}
          state={connectFlow.state}
          onCancel={() => void connectFlow.cancel()}
          onClose={() => connectFlow.reset()}
        />
      )}
    </Card>
  );
}
