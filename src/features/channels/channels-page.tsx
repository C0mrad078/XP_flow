import { Plus, Radio } from "lucide-react";

import { PageContainer } from "@/components/common/page-container";
import { PageHeader } from "@/components/common/page-header";
import { EmptyState } from "@/components/feedback/empty-state";
import { Button } from "@/components/ui/button";
import { mockChannels } from "@/development/mock-data/channels";

import { ChannelCard } from "./channel-card";

export function ChannelsPage() {
  return (
    <PageContainer>
      <PageHeader
        title="Channels"
        description="Every channel in your network and its connection health across platforms."
        actions={
          <Button size="sm">
            <Plus />
            Add channel
          </Button>
        }
      />

      {mockChannels.length === 0 ? (
        <EmptyState
          icon={Radio}
          title="No channels connected"
          description="Add a channel to start organizing content and connecting platform accounts."
        />
      ) : (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
          {mockChannels.map((channel) => (
            <ChannelCard key={channel.id} channel={channel} />
          ))}
        </div>
      )}
    </PageContainer>
  );
}
