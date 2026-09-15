import { CalendarClock, Pause, Play, Plus } from "lucide-react";

import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { PlatformBadge } from "@/components/ui/platform-badge";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { useSetChannelStatus } from "@/hooks/use-channels";
import { useCreatePlatformAccount, usePlatformAccounts } from "@/hooks/use-platform-accounts";
import { useQueueList } from "@/hooks/use-queue";
import { useScheduleSlots } from "@/hooks/use-schedule-slots";
import { cn } from "@/lib/utilities/cn";
import type { Channel } from "@/lib/tauri";
import { PLATFORMS, PLATFORM_LABELS, type Platform } from "@/types/domain";

export interface ChannelCardProps {
  channel: Channel;
  onOpenSchedule: (channelId: string) => void;
}

export function ChannelCard({ channel, onOpenSchedule }: ChannelCardProps) {
  const { data: accounts = [] } = usePlatformAccounts(channel.id);
  const { data: slots = [] } = useScheduleSlots(channel.id);
  const { data: queuePage } = useQueueList({
    channel_id: channel.id,
    statuses: ["queued", "scheduled"],
    page_size: 1,
  });
  const setStatus = useSetChannelStatus();
  const createPlatformAccount = useCreatePlatformAccount(channel.id);
  const availablePlatforms = PLATFORMS.filter((p) => !accounts.some((a) => a.platform === p));

  const activeSlotsPerWeek = slots.filter((s) => s.is_active).length;
  const queuedCount = queuePage?.total ?? 0;
  const contentStockDays =
    activeSlotsPerWeek > 0 ? Math.floor(queuedCount / (activeSlotsPerWeek / 7)) : queuedCount > 0 ? null : 0;
  const isPaused = channel.status === "paused";

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
        <div className="flex flex-wrap items-center gap-2">
          {accounts.map((account) => (
            <PlatformBadge key={account.id} platform={account.platform} size="sm" />
          ))}
          {availablePlatforms.length > 0 && (
            <Select value="" onValueChange={(value) => createPlatformAccount.mutate(value as Platform)}>
              <SelectTrigger className="h-7 w-8 justify-center border-dashed p-0 [&>svg]:hidden">
                <Plus className="size-3.5" />
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {availablePlatforms.map((p) => (
                  <SelectItem key={p} value={p}>
                    {PLATFORM_LABELS[p]}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          )}
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
    </Card>
  );
}
