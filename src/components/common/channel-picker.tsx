import { useState } from "react";
import { Plus } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { useChannels, useCreateChannel } from "@/hooks/use-channels";
import { cn } from "@/lib/utilities/cn";

const UNASSIGNED_VALUE = "__unassigned__";
const NEW_CHANNEL_VALUE = "__new__";

export interface ChannelPickerProps {
  value: string | null | undefined;
  onChange: (channelId: string | null) => void;
  allowUnassigned?: boolean;
  className?: string;
}

/** Shared channel selector (section 46: Content Details, context menu,
 * bulk actions all need this) backed by the real `channels` table
 * (section 9's source→channel mapping needs real UUIDs, not the Phase 1
 * mock Channels list). Includes an inline "New channel" quick-add so a
 * user isn't blocked by an empty list on first use. */
export function ChannelPicker({ value, onChange, allowUnassigned = true, className }: ChannelPickerProps) {
  const { data: channels = [] } = useChannels();
  const createChannel = useCreateChannel();
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");

  async function handleValueChange(next: string) {
    if (next === NEW_CHANNEL_VALUE) {
      setCreating(true);
      return;
    }
    onChange(next === UNASSIGNED_VALUE ? null : next);
  }

  async function submitNewChannel() {
    const trimmed = newName.trim();
    if (!trimmed) return;
    const channel = await createChannel.mutateAsync(trimmed);
    onChange(channel.id);
    setCreating(false);
    setNewName("");
  }

  if (creating) {
    return (
      <form
        className={cn("flex items-center gap-1.5", className)}
        onSubmit={(e) => {
          e.preventDefault();
          submitNewChannel();
        }}
      >
        <Input
          autoFocus
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
          placeholder="New channel name"
          className="h-8"
        />
        <Button type="submit" size="sm" disabled={!newName.trim() || createChannel.isPending}>
          Add
        </Button>
        <Button type="button" variant="ghost" size="sm" onClick={() => setCreating(false)}>
          Cancel
        </Button>
      </form>
    );
  }

  return (
    <Select value={value ?? UNASSIGNED_VALUE} onValueChange={handleValueChange}>
      <SelectTrigger className={cn("h-8", className)}>
        <SelectValue placeholder="Unassigned" />
      </SelectTrigger>
      <SelectContent>
        {allowUnassigned && <SelectItem value={UNASSIGNED_VALUE}>Unassigned</SelectItem>}
        {channels.map((channel) => (
          <SelectItem key={channel.id} value={channel.id}>
            {channel.name}
          </SelectItem>
        ))}
        <SelectItem value={NEW_CHANNEL_VALUE}>
          <span className="flex items-center gap-1.5 text-primary">
            <Plus className="size-3.5" />
            New channel
          </span>
        </SelectItem>
      </SelectContent>
    </Select>
  );
}
