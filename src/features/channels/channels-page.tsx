import { useState } from "react";
import { Plus, Radio } from "lucide-react";

import { PageContainer } from "@/components/common/page-container";
import { PageHeader } from "@/components/common/page-header";
import { EmptyState } from "@/components/feedback/empty-state";
import { ErrorState } from "@/components/feedback/error-state";
import { LoadingState } from "@/components/feedback/loading-state";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { useChannelOverview, useCreateChannel } from "@/hooks/use-channels";
import { toast } from "@/stores/toast-store";
import { isAppError } from "@/types/domain";

import { ChannelCard } from "./channel-card";
import { ChannelScheduleDrawer } from "./channel-schedule-drawer";

export function ChannelsPage() {
  const { data: overviews = [], isLoading, isError, refetch } = useChannelOverview();
  const createChannel = useCreateChannel();
  const [addOpen, setAddOpen] = useState(false);
  const [name, setName] = useState("");
  const [scheduleChannelId, setScheduleChannelId] = useState<string | null>(null);

  function handleSubmit() {
    const trimmed = name.trim();
    if (!trimmed) return;
    createChannel.mutate(trimmed, {
      onSuccess: () => {
        setAddOpen(false);
        setName("");
      },
      onError: (error) =>
        toast({
          variant: "error",
          title: "Couldn't create channel",
          description: isAppError(error) ? error.user_message : undefined,
        }),
    });
  }

  return (
    <PageContainer>
      <PageHeader
        title="Channels"
        description="Every channel in your network and its connection health across platforms."
        actions={
          <Button size="sm" onClick={() => setAddOpen(true)}>
            <Plus />
            Add channel
          </Button>
        }
      />

      <Dialog open={addOpen} onOpenChange={setAddOpen}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Add channel</DialogTitle>
          </DialogHeader>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              handleSubmit();
            }}
          >
            <Input
              autoFocus
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Channel name"
            />
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => setAddOpen(false)}>
                Cancel
              </Button>
              <Button type="submit" disabled={!name.trim() || createChannel.isPending}>
                Add channel
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>

      {isLoading && <LoadingState label="Loading channels…" />}
      {isError && <ErrorState onRetry={() => refetch()} />}

      {overviews.length === 0 && !isLoading ? (
        <EmptyState
          icon={Radio}
          title="No channels connected"
          description="Add a channel to start organizing content and connecting platform accounts."
        />
      ) : (
        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3">
          {overviews.map((overview) => (
            <ChannelCard
              key={overview.channel.id}
              overview={overview}
              onOpenSchedule={setScheduleChannelId}
            />
          ))}
        </div>
      )}

      <ChannelScheduleDrawer
        channelId={scheduleChannelId}
        channelName={overviews.find((o) => o.channel.id === scheduleChannelId)?.channel.name ?? ""}
        onClose={() => setScheduleChannelId(null)}
      />
    </PageContainer>
  );
}
