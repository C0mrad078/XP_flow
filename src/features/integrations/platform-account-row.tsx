import { CheckCircle2, RefreshCw, Unlink, XCircle } from "lucide-react";

import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { PlatformBadge } from "@/components/ui/platform-badge";
import { useDisconnectPlatformAccount, useValidatePlatformAccount } from "@/hooks/use-platform-auth";
import { formatRelativeTime } from "@/lib/formatting/date";
import { cn } from "@/lib/utilities/cn";
import { confirmAction } from "@/stores/confirm-store";
import { toast } from "@/stores/toast-store";
import { isAppError } from "@/types/domain";
import {
  CAPABILITY_LABELS,
  CONNECTION_HEALTH_LABELS,
  deriveConnectionHealth,
  type PlatformAccount,
} from "@/types/platform-auth";

const HEALTH_DOT: Record<string, string> = {
  healthy: "bg-success",
  token_expiring: "bg-warning",
  permission_missing: "bg-warning",
  refresh_required: "bg-danger",
  disconnected: "bg-muted",
  provider_error: "bg-danger",
};

export interface PlatformAccountRowProps {
  account: PlatformAccount;
  channelName?: string;
  onReconnect: (account: PlatformAccount) => void;
}

export function PlatformAccountRow({ account, channelName, onReconnect }: PlatformAccountRowProps) {
  const health = deriveConnectionHealth(account);
  const validate = useValidatePlatformAccount();
  const disconnect = useDisconnectPlatformAccount();

  async function handleDisconnect() {
    const confirmed = await confirmAction({
      title: `Disconnect ${account.display_name ?? "this account"}?`,
      description:
        "Your scheduled publications will remain in XP FLOW, but they cannot be published until an account is connected again.",
      confirmLabel: "Disconnect",
      destructive: true,
    });
    if (!confirmed) return;
    disconnect.mutate(account.id, {
      onError: (error) =>
        toast({
          variant: "error",
          title: "Couldn't disconnect",
          description: isAppError(error) ? error.user_message : undefined,
        }),
    });
  }

  const needsReconnect = health === "refresh_required" || health === "disconnected";

  return (
    <div className="flex items-center gap-3 rounded-lg border border-border bg-surface p-3">
      <Avatar className="size-9">
        {account.avatar_url && <AvatarImage src={account.avatar_url} alt="" />}
        <AvatarFallback className="text-xs">
          {(account.display_name ?? account.platform).slice(0, 2).toUpperCase()}
        </AvatarFallback>
      </Avatar>

      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <PlatformBadge platform={account.platform} size="sm" iconOnly />
          <p className="truncate text-body-small font-medium text-foreground">
            {account.display_name ?? "Unnamed account"}
          </p>
        </div>
        <div className="mt-0.5 flex flex-wrap items-center gap-x-2 gap-y-1 text-caption normal-case tracking-normal">
          {channelName && <span>{channelName}</span>}
          {account.username_or_handle && <span>{account.username_or_handle}</span>}
          <span className="flex items-center gap-1">
            <span className={cn("size-1.5 rounded-full", HEALTH_DOT[health])} />
            {CONNECTION_HEALTH_LABELS[health]}
          </span>
          {account.last_validated_at && <span>Checked {formatRelativeTime(account.last_validated_at)}</span>}
        </div>
        <div className="mt-1.5 flex flex-wrap gap-1">
          {account.capabilities.length === 0 && (
            <Badge variant="outline" className="px-1.5 py-0 text-[0.625rem]">
              No permissions yet
            </Badge>
          )}
          {account.capabilities.map((capability) => (
            <Badge key={capability} variant="outline" className="gap-1 px-1.5 py-0 text-[0.625rem]">
              <CheckCircle2 className="size-2.5 text-success" />
              {CAPABILITY_LABELS[capability]}
            </Badge>
          ))}
          {!account.capabilities.includes("upload_video") && (
            <Badge variant="outline" className="gap-1 px-1.5 py-0 text-[0.625rem] text-muted-foreground">
              <XCircle className="size-2.5" />
              Publishing not enabled yet
            </Badge>
          )}
        </div>
      </div>

      <div className="flex shrink-0 items-center gap-1">
        {needsReconnect ? (
          <Button size="sm" variant="outline" onClick={() => onReconnect(account)}>
            Reconnect
          </Button>
        ) : (
          <Button
            size="sm"
            variant="ghost"
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
            <RefreshCw className={cn("size-3.5", validate.isPending && "animate-spin")} />
          </Button>
        )}
        <Button size="sm" variant="ghost" onClick={handleDisconnect} disabled={disconnect.isPending}>
          <Unlink className="size-3.5" />
        </Button>
      </div>
    </div>
  );
}
