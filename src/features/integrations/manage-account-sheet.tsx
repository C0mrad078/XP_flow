import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { useChannels } from "@/hooks/use-channels";
import { useReassignPlatformAccountChannel } from "@/hooks/use-platform-accounts";
import { useDisconnectPlatformAccount, useValidatePlatformAccount } from "@/hooks/use-platform-auth";
import { formatDateTime, formatRelativeTime } from "@/lib/formatting/date";
import { confirmAction } from "@/stores/confirm-store";
import { toast } from "@/stores/toast-store";
import { isAppError } from "@/types/domain";
import {
  CAPABILITY_LABELS,
  CONNECTION_HEALTH_LABELS,
  deriveConnectionHealth,
  type PlatformAccount,
} from "@/types/platform-auth";

export interface ManageAccountSheetProps {
  account: PlatformAccount | null;
  onClose: () => void;
  onReconnect: (account: PlatformAccount) => void;
}

/** Section 50's "Manage" view — profile, connection, permissions,
 * assigned channel, token health, and the Reconnect/Disconnect actions.
 * Never renders a raw token: only expiry timestamps and validation
 * history, which is all `PlatformAccount` ever carries. */
export function ManageAccountSheet({ account, onClose, onReconnect }: ManageAccountSheetProps) {
  const { data: channels = [] } = useChannels();
  const reassign = useReassignPlatformAccountChannel();
  const validate = useValidatePlatformAccount();
  const disconnect = useDisconnectPlatformAccount();

  if (!account) return null;
  const health = deriveConnectionHealth(account);

  async function handleDisconnect() {
    if (!account) return;
    const confirmed = await confirmAction({
      title: `Disconnect ${account.display_name ?? "this account"}?`,
      description:
        "Your scheduled publications will remain in XP FLOW, but they cannot be published until an account is connected again.",
      confirmLabel: "Disconnect",
      destructive: true,
    });
    if (!confirmed) return;
    disconnect.mutate(account.id, {
      onSuccess: onClose,
      onError: (error) =>
        toast({
          variant: "error",
          title: "Couldn't disconnect",
          description: isAppError(error) ? error.user_message : undefined,
        }),
    });
  }

  return (
    <Sheet open onOpenChange={(open) => !open && onClose()}>
      <SheetContent className="overflow-y-auto">
        <SheetHeader>
          <SheetTitle>{account.display_name ?? "Unnamed account"}</SheetTitle>
          <SheetDescription>{CONNECTION_HEALTH_LABELS[health]}</SheetDescription>
        </SheetHeader>

        <div className="flex items-center gap-3">
          <Avatar className="size-12">
            {account.avatar_url && <AvatarImage src={account.avatar_url} alt="" />}
            <AvatarFallback>
              {(account.display_name ?? account.platform).slice(0, 2).toUpperCase()}
            </AvatarFallback>
          </Avatar>
          <div className="min-w-0">
            <p className="truncate text-body font-medium text-foreground">{account.display_name ?? "—"}</p>
            {account.username_or_handle && (
              <p className="truncate text-body-small text-muted-foreground">{account.username_or_handle}</p>
            )}
          </div>
        </div>

        <div className="border-t border-border" />

        <section className="flex flex-col gap-2">
          <p className="text-caption">Permissions granted</p>
          <div className="flex flex-wrap gap-1.5">
            {account.capabilities.length === 0 && (
              <span className="text-body-small text-muted-foreground">None yet</span>
            )}
            {account.capabilities.map((capability) => (
              <Badge key={capability} variant="outline">
                {CAPABILITY_LABELS[capability]}
              </Badge>
            ))}
          </div>
        </section>

        <div className="border-t border-border" />

        <section className="flex flex-col gap-2">
          <p className="text-caption">Assigned channel</p>
          <Select
            value={account.channel_id}
            onValueChange={(channelId) =>
              reassign.mutate(
                { id: account.id, newChannelId: channelId },
                {
                  onError: (error) =>
                    toast({
                      variant: "error",
                      title: "Couldn't reassign channel",
                      description: isAppError(error) ? error.user_message : undefined,
                    }),
                },
              )
            }
          >
            <SelectTrigger className="w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {channels.map((channel) => (
                <SelectItem key={channel.id} value={channel.id}>
                  {channel.name}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </section>

        <div className="border-t border-border" />

        <section className="flex flex-col gap-1.5 text-body-small">
          <p className="text-caption">Token health</p>
          <DetailRow
            label="Access token expires"
            value={account.access_expires_at ? formatDateTime(account.access_expires_at) : "—"}
          />
          <DetailRow
            label="Refresh token expires"
            value={account.refresh_expires_at ? formatDateTime(account.refresh_expires_at) : "Doesn't expire"}
          />
          <DetailRow
            label="Last validated"
            value={account.last_validated_at ? formatRelativeTime(account.last_validated_at) : "Never"}
          />
          <DetailRow
            label="Last refreshed"
            value={account.last_refreshed_at ? formatRelativeTime(account.last_refreshed_at) : "Never"}
          />
          {account.last_error_message && (
            <DetailRow label="Last error" value={account.last_error_message} valueClassName="text-danger" />
          )}
        </section>

        <div className="border-t border-border" />

        <div className="mt-auto flex flex-col gap-2">
          <Button
            variant="outline"
            onClick={() =>
              validate.mutate(account.id, {
                onSuccess: () => toast({ variant: "success", title: "Connection verified" }),
                onError: (error) =>
                  toast({
                    variant: "error",
                    title: "Validation failed",
                    description: isAppError(error) ? error.user_message : undefined,
                  }),
              })
            }
            disabled={validate.isPending}
          >
            Validate connection
          </Button>
          <Button variant="outline" onClick={() => onReconnect(account)}>
            Reconnect
          </Button>
          <Button variant="destructive" onClick={handleDisconnect} disabled={disconnect.isPending}>
            Disconnect
          </Button>
        </div>
      </SheetContent>
    </Sheet>
  );
}

function DetailRow({
  label,
  value,
  valueClassName,
}: {
  label: string;
  value: string;
  valueClassName?: string;
}) {
  return (
    <div className="flex items-center justify-between gap-3">
      <span className="text-muted-foreground">{label}</span>
      <span className={valueClassName ?? "text-foreground"}>{value}</span>
    </div>
  );
}
