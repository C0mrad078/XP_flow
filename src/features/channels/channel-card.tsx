import { CheckCircle2, XCircle } from "lucide-react";

import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader } from "@/components/ui/card";
import { PlatformBadge } from "@/components/ui/platform-badge";
import type { ChannelHealth, MockChannel } from "@/development/mock-data/channels";
import { cn } from "@/lib/utilities/cn";
import { PLATFORMS } from "@/types/domain";

const HEALTH_LABEL: Record<ChannelHealth, string> = {
  healthy: "Healthy",
  attention: "Needs attention",
  critical: "Critical",
};

const HEALTH_CLASSES: Record<ChannelHealth, string> = {
  healthy: "bg-success",
  attention: "bg-warning",
  critical: "bg-danger",
};

export function ChannelCard({ channel }: { channel: MockChannel }) {
  return (
    <Card>
      <CardHeader className="flex-row items-start justify-between space-y-0">
        <div className="flex items-center gap-3">
          <Avatar className="size-10">
            <AvatarFallback className="text-sm">{channel.name.slice(0, 2).toUpperCase()}</AvatarFallback>
          </Avatar>
          <div>
            <p className="text-section-title text-foreground">{channel.name}</p>
            <p className="text-body-small text-muted-foreground">{channel.niche}</p>
          </div>
        </div>
        <div className="flex items-center gap-1.5" title={HEALTH_LABEL[channel.health]}>
          <span className={cn("size-2 rounded-full", HEALTH_CLASSES[channel.health])} aria-hidden />
          <span className="text-caption">{HEALTH_LABEL[channel.health]}</span>
        </div>
      </CardHeader>

      <CardContent className="flex flex-col gap-4">
        <div className="flex flex-wrap gap-2">
          {PLATFORMS.map((platform) => {
            const connection = channel.connections[platform];
            return (
              <div key={platform} className="flex items-center gap-1.5">
                <PlatformBadge platform={platform} size="sm" />
                {connection.connected ? (
                  <CheckCircle2 className="size-3.5 text-success" />
                ) : (
                  <XCircle className="size-3.5 text-muted" />
                )}
              </div>
            );
          })}
        </div>

        <div className="grid grid-cols-2 gap-3 border-t border-border pt-3">
          <div>
            <p className="text-metric-small text-foreground">{channel.queuedVideos}</p>
            <p className="text-caption">Queued videos</p>
          </div>
          <div>
            <p
              className={cn(
                "text-metric-small",
                channel.contentStockDays <= 2 ? "text-danger" : "text-foreground",
              )}
            >
              {channel.contentStockDays}d
            </p>
            <p className="text-caption">Content stock</p>
          </div>
        </div>

        {channel.contentStockDays <= 2 && (
          <Badge variant="danger" className="w-fit">
            Running low on content
          </Badge>
        )}
      </CardContent>
    </Card>
  );
}
