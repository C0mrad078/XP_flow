import { Pause, Play } from "lucide-react";

import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { PlatformBadge } from "@/components/ui/platform-badge";
import { useSetChannelStatus } from "@/hooks/use-channels";
import { usePlatformAccounts } from "@/hooks/use-platform-accounts";
import { useQueueList } from "@/hooks/use-queue";
import { useScheduleSlots } from "@/hooks/use-schedule-slots";
import { cn } from "@/lib/utilities/cn";
import type { Channel } from "@/lib/tauri";

export function ChannelCard({ channel }: { channel: Channel }) {
  const { data: accounts = [] } = usePlatformAccounts(channel.id);
  const { data: slots = [] } = useScheduleSlots(channel.id);
  const { data: queuePage } = useQueueList({
    channel_id: channel.id,
    statuses: ["queued", "scheduled"],
    page_size: 1,
  });
  const setStatus = useSetChannelStatus();

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
        <Button
          variant="ghost"
          size="sm"
          onClick={() => setStatus.mutate({ id: channel.id, status: isPaused ? "active" : "paused" })}
          disabled={setStatus.isPending}
        >
          {isPaused ? <Play className="size-3.5" /> : <Pause className="size-3.5" />}
          {isPaused ? "Paused" : "Active"}
        </Button>
      </CardHeader>

      <CardContent className="flex flex-col gap-4">
        <div className="flex flex-wrap gap-2">
          {accounts.length === 0 ? (
            <p className="text-caption normal-case tracking-normal">No platform targets configured yet</p>
          ) : (
            accounts.map((account) => (
              <PlatformBadge key={account.id} platform={account.platform} size="sm" />
            ))
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
